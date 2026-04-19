use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tokio::time::sleep;

use crate::account::AccountRepository;
use crate::status_sync;
use crate::AppState;

pub async fn start_scheduler(handle: AppHandle) {
    let db_state = handle.state::<AppState>().db.clone();

    loop {
        let interval_seconds = load_status_check_interval_seconds(&db_state).await;
        sleep(Duration::from_secs(interval_seconds)).await;

        let account_ids = {
            let db = db_state.lock().await;
            let repo = AccountRepository::new(&db);

            match repo.list_all() {
                Ok(accounts) => accounts.into_iter().map(|account| account.id).collect::<Vec<_>>(),
                Err(_) => Vec::new(),
            }
        };

        let app_state = handle.state::<AppState>();

        for account_id in account_ids {
            let (account, credential) =
                match status_sync::load_account_and_credential(app_state.inner(), &account_id).await {
                    Ok(pair) => pair,
                    Err(_) => continue,
                };
            let outcome = match status_sync::evaluate_account_refresh(&account, &credential).await {
                Ok(outcome) => outcome,
                Err(_) => continue,
            };

            let refreshed_account = {
                let db = db_state.lock().await;
                let repo = AccountRepository::new(&db);
                match status_sync::persist_refresh_outcome(&repo, &account.id, &outcome) {
                    Ok(account) => account,
                    Err(_) => continue,
                }
            };

            let _ = handle.emit(
                "account-status-updated",
                serde_json::json!({
                    "account_id": refreshed_account.id,
                    "status": refreshed_account.status.to_string(),
                    "message": refreshed_account.status_message,
                    "account": refreshed_account,
                }),
            );
        }
    }
}

async fn load_status_check_interval_seconds(
    db_state: &std::sync::Arc<tokio::sync::Mutex<crate::storage::Database>>,
) -> u64 {
    let db = db_state.lock().await;
    let conn = db.get_conn();
    let raw_value: Result<String, rusqlite::Error> = conn.query_row(
        "SELECT value FROM settings WHERE key = 'check_interval' LIMIT 1",
        [],
        |row| row.get(0),
    );

    // 调度器必须有稳定兜底值，避免设置损坏时把后台检测完全停掉或拉成极短轮询。
    match raw_value {
        Ok(value) => parse_interval_seconds(&value).unwrap_or(300),
        Err(rusqlite::Error::QueryReturnedNoRows) => 300,
        Err(_) => 300,
    }
}

fn parse_interval_seconds(value: &str) -> Option<u64> {
    let seconds = value.trim().parse::<u64>().ok()?;
    if seconds == 0 {
        return None;
    }

    Some(seconds)
}
