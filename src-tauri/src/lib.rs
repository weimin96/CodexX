pub mod account;
pub mod auth;
pub mod codex_runtime;
pub mod codex_usage;
pub mod commands;
pub mod error;
pub mod local_sync;
pub mod scheduler;
pub mod security;
pub mod status_sync;
pub mod storage;
pub mod usage;

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
            // 初始化数据库，所有账号凭证后续都通过仓储层加密写入。
            let app_dir = app
                .path()
                .app_data_dir()
                .expect("Failed to get app data dir");
            std::fs::create_dir_all(&app_dir).expect("Failed to create app dir");

            let db_path = app_dir.join("codex.db");
            let db =
                Database::new(db_path.to_str().unwrap()).expect("Failed to initialize database");

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
            commands::account::export_accounts,
            commands::account::import_accounts,
            commands::account::sync_local_auth_file,
            // Codex 启动命令
            commands::codex::run_codex_exec_session,
            commands::codex::open_codex_interactive_session,
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
