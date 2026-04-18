use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tokio::time::sleep;

use crate::account::{AccountRepository, AccountStatus};
use crate::auth::AuthService;
use crate::AppState;

pub async fn start_scheduler(handle: AppHandle) {
    let db_state = handle.state::<AppState>().db.clone();

    loop {
        sleep(Duration::from_secs(300)).await; // 5 minutes

        let accounts_with_credentials = {
            let db = db_state.lock().await;
            let repo = AccountRepository::new(&db);

            match repo.list_all() {
                Ok(accounts) => {
                    let mut pairs = Vec::new();
                    for account in accounts {
                        if let Ok(credential) = repo.get_credential(&account.id) {
                            pairs.push((account, credential));
                        }
                    }
                    pairs
                }
                Err(_) => Vec::new(),
            }
        };

        let auth_service = AuthService::new();

        for (account, credential) in accounts_with_credentials {
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

                {
                    let db = db_state.lock().await;
                    let repo = AccountRepository::new(&db);
                    let _ = repo.update_status(
                        &account.id,
                        &new_status,
                        auth_result.message.as_deref(),
                    );
                }

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
