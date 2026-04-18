pub mod account;
pub mod auth;
pub mod commands;
pub mod error;
pub mod local_sync;
pub mod scheduler;
pub mod security;
pub mod storage;
pub mod usage;

use std::sync::Arc;
use storage::Database;
use tauri::Manager;
use tokio::sync::Mutex;

pub struct AppState {
    pub db: Arc<Mutex<Database>>,
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
            // Initialize database
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
            });

            // Setup system tray
            #[cfg(all(desktop))]
            {
                let handle = app.handle().clone();
                setup_tray(handle)?;
            }

            // Start background scheduler
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                scheduler::start_scheduler(handle).await;
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Account commands
            commands::account::create_account,
            commands::account::update_account,
            commands::account::delete_account,
            commands::account::list_accounts,
            commands::account::get_account,
            commands::account::switch_account,
            commands::account::set_default_account,
            commands::account::export_accounts,
            commands::account::import_accounts,
            commands::account::sync_local_auth_file,
            // Auth commands
            commands::auth::refresh_token,
            commands::auth::validate_token,
            commands::auth::get_auth_status,
            // Status commands
            commands::status::check_status,
            commands::status::check_all_status,
            // Usage commands
            commands::usage::fetch_usage,
            commands::usage::get_usage_stats,
            commands::usage::get_usage_chart_data,
            // Settings commands
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
