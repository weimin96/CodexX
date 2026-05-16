pub mod account;
pub mod auth;
pub mod codex_config;
pub mod codex_runtime;
pub mod codex_session_import;
pub mod codex_token_usage;
pub mod codex_usage;
pub mod commands;
pub mod error;
pub mod local_sync;
pub mod scheduler;
pub mod security;
pub mod status_sync;
pub mod storage;
pub mod usage;

use rusqlite::OptionalExtension;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::thread::JoinHandle;
use storage::Database;
use tauri::{Emitter, Manager};
use tokio::sync::Mutex;

use crate::account::{Account, AccountRepository};
use crate::auth::PendingOAuthLogin;
use crate::codex_runtime::{close_codex_desktop_app, open_codex_desktop_app};
use crate::local_sync::LocalAuthSyncService;

#[cfg(all(desktop))]
const TRAY_ICON_BYTES: &[u8] = include_bytes!("../icons/64x64.png");
#[cfg(all(desktop))]
const MAIN_TRAY_ID: &str = "main";
const MAIN_WINDOW_LABEL: &str = "main";
#[cfg(all(desktop))]
const TRAY_MENU_OPEN_ID: &str = "open";
#[cfg(all(desktop))]
const TRAY_MENU_RESTART_CODEX_APP_ID: &str = "restart-codex-app";
#[cfg(all(desktop))]
const TRAY_MENU_QUIT_ID: &str = "quit";
#[cfg(all(desktop))]
const TRAY_MENU_SWITCH_ACCOUNT_PREFIX: &str = "switch-account::";
#[cfg(all(desktop))]
const TRAY_DEFAULT_ACCOUNT_UPDATED_EVENT: &str = "default-account-updated";
#[cfg(all(desktop))]
const TRAY_REMAINING_QUOTA_EPSILON: f64 = 0.000_001;
const WINDOW_CLOSE_ACTION_SETTING_KEY: &str = "window_close_action";
const WINDOW_CLOSE_ACTION_TRAY: &str = "tray";
const WINDOW_CLOSE_ACTION_QUIT: &str = "quit";
pub const APP_USER_AGENT: &str = concat!("codexx/", env!("CARGO_PKG_VERSION"));

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
        .on_window_event(|window, event| {
            if window.label() != MAIN_WINDOW_LABEL {
                return;
            }

            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                handle_main_window_close_requested(window, api);
            }
        })
        .setup(|app| {
            // 数据库跟随 Codex 主目录，便于账号管理数据和 Codex 本地状态一起备份。
            let legacy_app_dir = app.path().app_data_dir().expect("无法获取旧应用数据目录");
            let legacy_db_path = legacy_app_dir.join("codexX.db");
            let db_path = resolve_codex_manager_db_path().expect("无法解析数据库目录");
            migrate_legacy_database(&legacy_db_path, &db_path).expect("迁移旧数据库失败");
            let db = Database::new(&db_path).expect("初始化数据库失败");
            let initial_tray_accounts =
                AccountRepository::new(&db)
                    .list_all()
                    .unwrap_or_else(|error| {
                        log::warn!("读取托盘初始账号列表失败: {error}");
                        Vec::new()
                    });

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
                setup_tray(handle, &initial_tray_accounts)?;
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
            commands::codex::close_codex_app,
            commands::codex_config::read_codex_config_file,
            commands::codex_config::save_codex_config_field,
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
            commands::usage::rebuild_account_usage,
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
    let app_dir = PathBuf::from(user_home).join(".codex").join("CodexX");
    std::fs::create_dir_all(&app_dir)?;

    Ok(app_dir.join("codexX.db"))
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
fn setup_tray(handle: tauri::AppHandle, accounts: &[Account]) -> tauri::Result<()> {
    use tauri::image::Image;
    use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

    let menu = build_tray_menu(&handle, accounts)?;
    let tray_icon = Image::from_bytes(TRAY_ICON_BYTES)?;

    TrayIconBuilder::with_id(MAIN_TRAY_ID)
        .icon(tray_icon)
        .icon_as_template(false)
        .menu(&menu)
        .tooltip("CodexX")
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            TRAY_MENU_OPEN_ID => open_main_window(app),
            TRAY_MENU_RESTART_CODEX_APP_ID => restart_codex_app_from_tray(),
            TRAY_MENU_QUIT_ID => {
                app.exit(0);
            }
            menu_id
                if let Some(account_id) = menu_id.strip_prefix(TRAY_MENU_SWITCH_ACCOUNT_PREFIX) =>
            {
                switch_account_from_tray(app.clone(), account_id.to_string());
            }
            unknown_id => {
                log::warn!("忽略未知托盘菜单事件: {unknown_id}");
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                open_main_window(tray.app_handle());
            }
        })
        .build(&handle)?;

    Ok(())
}

#[cfg(all(desktop))]
pub async fn refresh_tray_menu(app: &tauri::AppHandle, db: &Arc<Mutex<Database>>) {
    let accounts = {
        let db = db.lock().await;
        let repo = AccountRepository::new(&db);
        match repo.list_all() {
            Ok(accounts) => accounts,
            Err(error) => {
                log::warn!("读取托盘账号列表失败: {error}");
                return;
            }
        }
    };

    if let Err(error) = apply_tray_menu(app, &accounts) {
        log::warn!("刷新托盘菜单失败: {error}");
    }
}

#[cfg(not(all(desktop)))]
pub async fn refresh_tray_menu(_app: &tauri::AppHandle, _db: &Arc<Mutex<Database>>) {}

#[cfg(all(desktop))]
fn apply_tray_menu(app: &tauri::AppHandle, accounts: &[Account]) -> tauri::Result<()> {
    let Some(tray) = app.tray_by_id(MAIN_TRAY_ID) else {
        return Ok(());
    };

    tray.set_menu(Some(build_tray_menu(app, accounts)?))?;
    tray.set_show_menu_on_left_click(false)?;
    Ok(())
}

#[cfg(all(desktop))]
fn build_tray_menu<R: tauri::Runtime, M: tauri::Manager<R>>(
    manager: &M,
    accounts: &[Account],
) -> tauri::Result<tauri::menu::Menu<R>> {
    use tauri::menu::{CheckMenuItem, IsMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};

    let open = MenuItem::with_id(manager, TRAY_MENU_OPEN_ID, "打开", true, None::<&str>)?;
    let restart = MenuItem::with_id(
        manager,
        TRAY_MENU_RESTART_CODEX_APP_ID,
        "重启 Codex App",
        true,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(manager, TRAY_MENU_QUIT_ID, "退出", true, None::<&str>)?;
    let first_separator = PredefinedMenuItem::separator(manager)?;
    let second_separator = PredefinedMenuItem::separator(manager)?;

    let tray_accounts = accounts
        .iter()
        .filter(|account| is_tray_switchable_account(account))
        .collect::<Vec<_>>();

    let switch_account_submenu = if tray_accounts.is_empty() {
        let empty = MenuItem::with_id(
            manager,
            "switch-account-empty",
            "暂无可用账号",
            false,
            None::<&str>,
        )?;
        Submenu::with_items(manager, "切换账号", false, &[&empty])?
    } else {
        let switch_items = tray_accounts
            .iter()
            .map(|account| {
                CheckMenuItem::with_id(
                    manager,
                    format!("{TRAY_MENU_SWITCH_ACCOUNT_PREFIX}{}", account.id),
                    tray_account_menu_label(account),
                    true,
                    account.is_default,
                    None::<&str>,
                )
            })
            .collect::<tauri::Result<Vec<_>>>()?;
        let switch_refs = switch_items
            .iter()
            .map(|item| item as &dyn IsMenuItem<R>)
            .collect::<Vec<_>>();
        Submenu::with_items(manager, "切换账号", true, &switch_refs)?
    };

    Menu::with_items(
        manager,
        &[
            &open,
            &switch_account_submenu,
            &first_separator,
            &restart,
            &second_separator,
            &quit,
        ],
    )
}

#[cfg(all(desktop))]
fn tray_account_menu_label(account: &Account) -> String {
    let display_name = account
        .email
        .as_deref()
        .map(str::trim)
        .filter(|email| !email.is_empty())
        .unwrap_or(&account.name);

    let plan_label = resolve_tray_plan_label(account).to_ascii_uppercase();
    let remaining_five_hour = resolve_remaining_percent(account.codex_usage_5h.as_ref());
    let remaining_one_week = resolve_remaining_percent(account.codex_usage_week.as_ref());

    format!(
        "{display_name} [{plan_label}]  5h {remaining_five_hour:>3}%  7d {remaining_one_week:>3}%"
    )
}

#[cfg(all(desktop))]
fn is_tray_switchable_account(account: &Account) -> bool {
    let Some(five_hour) = account.codex_usage_5h.as_ref() else {
        return false;
    };
    let Some(one_week) = account.codex_usage_week.as_ref() else {
        return false;
    };

    // 需求口径：仅展示“剩余额度都大于 0”的账号。
    // 当前仅持有已用百分比 used_percent，因此用剩余百分比（100 - used_percent）判断。
    let remaining_five_hour = 100.0 - normalize_percent(five_hour.used_percent);
    let remaining_one_week = 100.0 - normalize_percent(one_week.used_percent);

    remaining_five_hour > TRAY_REMAINING_QUOTA_EPSILON
        && remaining_one_week > TRAY_REMAINING_QUOTA_EPSILON
}

#[cfg(all(desktop))]
fn resolve_tray_plan_label(account: &Account) -> &'static str {
    let normalized = account
        .codex_plan_type
        .as_deref()
        .unwrap_or("free")
        .trim()
        .to_ascii_lowercase();

    // 托盘展示需要稳定枚举值，避免因为上游新增文本导致菜单文案不可控。
    if normalized.contains("pro") {
        "pro"
    } else if normalized.contains("plus") {
        "plus"
    } else {
        "free"
    }
}

#[cfg(all(desktop))]
fn resolve_remaining_percent(window: Option<&crate::account::CodexUsageWindow>) -> i64 {
    let Some(window) = window else {
        return 0;
    };

    let used_percent = normalize_percent(window.used_percent);
    let remaining = (100.0 - used_percent).max(0.0).min(100.0);
    remaining.round() as i64
}

#[cfg(all(desktop))]
fn normalize_percent(value: f64) -> f64 {
    if !value.is_finite() {
        return 0.0;
    }
    value.max(0.0).min(100.0)
}

#[cfg(all(desktop))]
fn switch_account_from_tray(app: tauri::AppHandle, account_id: String) {
    let db = app.state::<AppState>().db.clone();
    tauri::async_runtime::spawn(async move {
        let switch_result = {
            let db = db.lock().await;
            let repo = AccountRepository::new(&db);
            LocalAuthSyncService::write_account_to_default_auth_file(&repo, &account_id)
        };

        match switch_result {
            Ok(account) => {
                refresh_tray_menu(&app, &db).await;
                let _ = app.emit(
                    TRAY_DEFAULT_ACCOUNT_UPDATED_EVENT,
                    serde_json::json!({ "account_id": account.id }),
                );
            }
            Err(error) => {
                log::warn!("托盘切换账号失败: {error}");
            }
        }
    });
}

#[cfg(all(desktop))]
fn restart_codex_app_from_tray() {
    std::thread::spawn(|| {
        match close_codex_desktop_app() {
            Ok(result) => {
                log::info!("托盘重启 Codex App 前关闭结果: {}", result.message);
            }
            Err(error) => {
                log::warn!("托盘重启 Codex App 前关闭失败: {error}");
                return;
            }
        }

        if let Err(error) = open_codex_desktop_app() {
            log::warn!("托盘重启 Codex App 失败: {error}");
        }
    });
}

#[cfg(all(desktop))]
fn open_main_window(app: &tauri::AppHandle) {
    let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) else {
        log::warn!("无法打开主窗口: main 窗口不存在");
        return;
    };

    restore_main_window(&window);
}

#[cfg(all(desktop))]
fn restore_main_window(window: &tauri::WebviewWindow) {
    log_window_operation("取消主窗口最小化", window.unminimize());
    log_window_operation("显示主窗口", window.show());
    log_window_operation("聚焦主窗口", window.set_focus());
}

fn handle_main_window_close_requested<R: tauri::Runtime>(
    window: &tauri::Window<R>,
    api: &tauri::CloseRequestApi,
) {
    match resolve_window_close_action(window.app_handle()) {
        WindowCloseAction::Tray => {
            api.prevent_close();

            // 关闭到托盘时保留进程与托盘菜单，便于用户稍后从托盘重新打开主窗口。
            if let Err(error) = window.hide() {
                log::warn!("隐藏主窗口失败，回退为直接退出: {error}");
                window.app_handle().exit(0);
            }
        }
        WindowCloseAction::Quit => {
            // 应用启用了托盘，仅关闭主窗口不会结束进程，因此这里显式退出整个应用。
            api.prevent_close();
            window.app_handle().exit(0);
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WindowCloseAction {
    Tray,
    Quit,
}

fn resolve_window_close_action<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> WindowCloseAction {
    let state = app.state::<AppState>();
    let db = state.db.blocking_lock();
    let conn = db.get_conn();

    match conn
        .query_row(
            "SELECT value FROM settings WHERE key = ?1",
            [WINDOW_CLOSE_ACTION_SETTING_KEY],
            |row| row.get::<_, String>(0),
        )
        .optional()
    {
        Ok(Some(value)) if value == WINDOW_CLOSE_ACTION_QUIT => WindowCloseAction::Quit,
        Ok(Some(value)) if value == WINDOW_CLOSE_ACTION_TRAY => WindowCloseAction::Tray,
        Ok(Some(value)) => {
            log::warn!("发现未知关闭窗口行为设置，回退为托盘模式: {value}");
            WindowCloseAction::Tray
        }
        Ok(None) => WindowCloseAction::Tray,
        Err(error) => {
            log::warn!("读取关闭窗口行为设置失败，回退为托盘模式: {error}");
            WindowCloseAction::Tray
        }
    }
}

#[cfg(all(desktop))]
fn log_window_operation(action: &str, result: tauri::Result<()>) {
    if let Err(error) = result {
        log::warn!("{action}失败: {error}");
    }
}
