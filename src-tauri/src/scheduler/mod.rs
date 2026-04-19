use chrono::Duration as ChronoDuration;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tokio::time::sleep;

use crate::account::{Account, AccountRepository, AccountStatus, AuthType};
use crate::auth::{self, AuthService};
use crate::error::AppError;
use crate::local_sync::LocalAuthSyncService;
use crate::status_sync;
use crate::{storage::Database, AppState};

const TOKEN_KEEPALIVE_MIN_AGE_MINUTES: i64 = 30;

pub async fn start_scheduler(handle: AppHandle) {
    let db_state = handle.state::<AppState>().db.clone();

    loop {
        let interval_seconds = load_status_check_interval_seconds(&db_state).await;
        let token_keepalive_enabled = load_token_keepalive_enabled(&db_state).await;
        sleep(Duration::from_secs(interval_seconds)).await;

        let account_ids = {
            let db = db_state.lock().await;
            let repo = AccountRepository::new(&db);

            match repo.list_all() {
                Ok(accounts) => accounts
                    .into_iter()
                    .map(|account| account.id)
                    .collect::<Vec<_>>(),
                Err(_) => Vec::new(),
            }
        };

        let app_state = handle.state::<AppState>();

        for account_id in account_ids {
            let (account, mut credential) = match status_sync::load_account_and_credential(
                app_state.inner(),
                &account_id,
            )
            .await
            {
                Ok(pair) => pair,
                Err(_) => continue,
            };

            if token_keepalive_enabled {
                match refresh_oauth_credential_if_due(app_state.inner(), &account, &credential)
                    .await
                {
                    Ok(Some(refreshed_credential)) => {
                        credential = refreshed_credential;
                    }
                    Ok(None) => {}
                    Err(error) => {
                        persist_token_keepalive_failure(
                            app_state.inner(),
                            &handle,
                            &account,
                            &error,
                        )
                        .await;
                    }
                }
            }

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

async fn refresh_oauth_credential_if_due(
    state: &AppState,
    account: &Account,
    credential: &str,
) -> Result<Option<String>, AppError> {
    if account.auth_type != AuthType::OAuthToken {
        return Ok(None);
    }

    if !auth::oauth_credential_refresh_due(
        credential,
        ChronoDuration::minutes(TOKEN_KEEPALIVE_MIN_AGE_MINUTES),
    ) {
        return Ok(None);
    }

    let auth_service = AuthService::new();
    let refresh_result = auth_service.refresh_oauth_credential(credential).await?;
    let db = state.db.lock().await;
    let repo = AccountRepository::new(&db);
    repo.update_credential(
        &account.id,
        &refresh_result.credential_value,
        Some("oauth_json"),
    )?;
    if account.is_default {
        LocalAuthSyncService::write_account_to_default_auth_file(&repo, &account.id)?;
    }

    Ok(Some(refresh_result.credential_value))
}

async fn persist_token_keepalive_failure(
    state: &AppState,
    handle: &AppHandle,
    account: &Account,
    error: &AppError,
) {
    let message = format!("Token 保活失败: {error}");
    let refreshed_account = {
        let db = state.db.lock().await;
        let repo = AccountRepository::new(&db);
        if repo
            .update_status(&account.id, &AccountStatus::Warning, Some(&message))
            .is_err()
        {
            return;
        }
        match repo.get_by_id(&account.id) {
            Ok(account) => account,
            Err(_) => return,
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

async fn load_status_check_interval_seconds(
    db_state: &std::sync::Arc<tokio::sync::Mutex<Database>>,
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

async fn load_token_keepalive_enabled(
    db_state: &std::sync::Arc<tokio::sync::Mutex<Database>>,
) -> bool {
    let db = db_state.lock().await;
    let conn = db.get_conn();
    let raw_value: Result<String, rusqlite::Error> = conn.query_row(
        "SELECT value FROM settings WHERE key = 'token_keepalive_enabled' LIMIT 1",
        [],
        |row| row.get(0),
    );

    match raw_value {
        Ok(value) => value == "true",
        Err(_) => false,
    }
}

fn parse_interval_seconds(value: &str) -> Option<u64> {
    let seconds = value.trim().parse::<u64>().ok()?;
    if seconds == 0 {
        return None;
    }

    Some(seconds)
}
