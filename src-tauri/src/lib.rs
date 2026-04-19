pub mod account;
pub mod auth;
pub mod codex_config;
pub mod codex_runtime;
pub mod codex_session_import;
pub mod codex_usage;
pub mod commands;
pub mod error;
pub mod local_sync;
pub mod scheduler;
pub mod security;
pub mod status_sync;
pub mod storage;
pub mod usage;

use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::thread::JoinHandle;
use storage::Database;
use tauri::Manager;
use tokio::sync::Mutex;

use crate::auth::PendingOAuthLogin;

pub struct OAuthCallbackListenerHandle {
    pub shutdown_tx: Option<Sender<()>>,
    pub task: Option<JoinHandle<()>>,
}

pub struct AppState {
    pub db: Arc<Mutex<Database>>,
    pub oauth_flow_lock: Arc<Mutex<()>>,
    pub pending_oauth_login: Mutex<Option<PendingOAuthLogin>>,
    pub oauth_listener: Mutex<Option<OAuthCallbackListenerHandle>>,
}

pub fn run() {
    env_logger::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            // 数据库跟随 Codex 主目录，便于账号管理数据和 Codex 本地状态一起备份。
            let legacy_app_dir = app.path().app_data_dir().expect("无法获取旧应用数据目录");
            let legacy_db_path = legacy_app_dir.join("codex.db");
            let db_path = resolve_codex_manager_db_path().expect("无法解析数据库目录");
            migrate_legacy_database(&legacy_db_path, &db_path).expect("迁移旧数据库失败");
            let db = Database::new(&db_path).expect("初始化数据库失败");

            app.manage(AppState {
                db: Arc::new(Mutex::new(db)),
                oauth_flow_lock: Arc::new(Mutex::new(())),
                pending_oauth_login: Mutex::new(None),
                oauth_listener: Mutex::new(None),
            });

            // 初始化系统托盘，保持主窗口关闭后的基础可达性。
            #[cfg(all(desktop))]
            {
                let handle = app.handle().clone();
                setup_tray(handle)?;
            }

            // 启动后台调度器，周期性刷新账号状态但不读取本地 auth.json。
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                scheduler::start_scheduler(handle).await;
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // 账号命令
            commands::account::create_account,
            commands::account::update_account,
            commands::account::delete_account,
            commands::account::list_accounts,
            commands::account::get_account,
            commands::account::get_account_credential,
            commands::account::switch_account,
            commands::account::set_default_account,
            commands::account::export_account_auth_file,
            commands::account::export_accounts,
            commands::account::import_accounts,
            commands::account::sync_local_auth_file,
            commands::account::sync_local_default_account,
            // Codex 启动命令
            commands::codex::run_codex_exec_session,
            commands::codex::trigger_codex_short_conversation,
            commands::codex::open_codex_interactive_session,
            commands::codex::launch_codex_cli,
            commands::codex::get_codex_launcher_config,
            commands::codex::launch_codex_app,
            commands::codex_config::read_codex_config_file,
            commands::codex_config::save_codex_config_file,
            // 认证命令
            commands::auth::refresh_token,
            commands::auth::validate_token,
            commands::auth::get_auth_status,
            commands::auth::prepare_oauth_login,
            commands::auth::open_oauth_login_url,
            commands::auth::complete_oauth_callback_login,
            commands::auth::cancel_oauth_login,
            // 状态命令
            commands::status::check_status,
            commands::status::check_all_status,
            // 用量命令
            commands::usage::fetch_usage,
            commands::usage::get_usage_stats,
            commands::usage::get_usage_chart_data,
            commands::usage::clear_usage_data,
            // 设置命令
            commands::settings::get_settings,
            commands::settings::save_settings,
            commands::settings::set_autostart,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn resolve_codex_manager_db_path() -> std::io::Result<PathBuf> {
    let user_home = resolve_user_home_dir()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "无法获取用户目录"))?;
    let app_dir = PathBuf::from(user_home).join(".codex").join("CodexManager");
    std::fs::create_dir_all(&app_dir)?;

    Ok(app_dir.join("codex.db"))
}

#[cfg(windows)]
fn resolve_user_home_dir() -> Option<std::ffi::OsString> {
    std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME"))
}

#[cfg(not(windows))]
fn resolve_user_home_dir() -> Option<std::ffi::OsString> {
    std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))
}

fn migrate_legacy_database(legacy_db_path: &Path, target_db_path: &Path) -> std::io::Result<()> {
    if target_db_path.exists() || !legacy_db_path.exists() || legacy_db_path == target_db_path {
        return Ok(());
    }

    if let Some(parent) = target_db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // 迁移采用复制而不是移动，避免首次启动失败时丢失旧应用数据目录中的数据库。
    std::fs::copy(legacy_db_path, target_db_path)?;
    copy_sqlite_sidecar_file(legacy_db_path, target_db_path, "wal")?;
    copy_sqlite_sidecar_file(legacy_db_path, target_db_path, "shm")?;
    Ok(())
}

fn copy_sqlite_sidecar_file(
    legacy_db_path: &Path,
    target_db_path: &Path,
    suffix: &str,
) -> std::io::Result<()> {
    let legacy_sidecar_path = legacy_db_path.with_extension(format!("db-{suffix}"));
    if !legacy_sidecar_path.exists() {
        return Ok(());
    }

    let target_sidecar_path = target_db_path.with_extension(format!("db-{suffix}"));
    std::fs::copy(legacy_sidecar_path, target_sidecar_path)?;
    Ok(())
}

#[cfg(all(desktop))]
fn setup_tray(handle: tauri::AppHandle) -> tauri::Result<()> {
    use tauri::menu::{Menu, MenuItem};
    use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

    let show = MenuItem::with_id(&handle, "show", "显示主窗口", true, None::<&str>)?;
    let quit = MenuItem::with_id(&handle, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(&handle, &[&show, &quit])?;

    TrayIconBuilder::new()
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        })
        .build(&handle)?;

    Ok(())
}
