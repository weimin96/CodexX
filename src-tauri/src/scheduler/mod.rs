use tauri::{AppHandle, Emitter};
use std::time::Duration;

use crate::account::{AccountRepository, AccountStatus};
use crate::auth::AuthService;
use crate::AppState;

pub async fn start_scheduler(handle: AppHandle) {
    let mut interval = tokio::time::interval(Duration::from_secs(300)); // 5 minutes

    loop {
        interval.tick().await;

        let state = handle.state::<AppState>();
        let db = state.db.lock().await;

        let repo = AccountRepository::new(&db);
        let auth_service = AuthService::new();

        if let Ok(accounts) = repo.list_all() {
            for account in accounts {
                let credential = match repo.get_credential(&account.id) {
                    Ok(c) => c,
                    Err(_) => continue,
                };

                let result = auth_service
                    .validate_credential(&account, &credential)
                    .await;

                if let Ok(auth_result) = result {
                    let new_status = match auth_result.status {
                        crate::auth::AuthStatus::Valid => AccountStatus::Normal,
                        crate::auth::AuthStatus::Expired => AccountStatus::Expired,
                        crate::auth::AuthStatus::Invalid => AccountStatus::Error,
                        crate::auth::AuthStatus::Unknown => AccountStatus::Unknown,
                    };

                    let _ = repo.update_status(
                        &account.id,
                        &new_status,
                        auth_result.message.as_deref(),
                    );

                    // Emit event to frontend
                    let _ = handle.emit(
                        "account-status-updated",
                        serde_json::json!({
                            "account_id": account.id,
                            "status": new_status.to_string(),
                            "message": auth_result.message,
                        }),
                    );
                }
            }
        }
    }
}
