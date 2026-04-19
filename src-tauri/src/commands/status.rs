use serde_json::Value;
use tauri::State;

use crate::account::{AccountRepository, AccountStatus};
use crate::error::AppError;
use crate::status_sync;
use crate::AppState;

#[tauri::command]
pub async fn check_status(
    state: State<'_, AppState>,
    account_id: String,
) -> Result<Value, AppError> {
    let (account, credential) =
        status_sync::load_account_and_credential(state.inner(), &account_id).await?;
    let outcome = status_sync::evaluate_account_refresh(&account, &credential).await?;

    {
        let db = state.db.lock().await;
        let repo = AccountRepository::new(&db);
        status_sync::persist_refresh_outcome(&repo, &account_id, &outcome)?;
    }

    Ok(serde_json::json!({
        "account_id": account_id,
        "status": outcome.status.to_string(),
        "message": outcome.message,
    }))
}

#[tauri::command]
pub async fn check_all_status(state: State<'_, AppState>) -> Result<Value, AppError> {
    let account_ids = {
        let db = state.db.lock().await;
        let repo = AccountRepository::new(&db);
        let accounts = repo.list_all()?;
        accounts
            .into_iter()
            .map(|account| account.id)
            .collect::<Vec<_>>()
    };

    let mut results = Vec::new();

    for account_id in account_ids {
        let (account, credential) =
            match status_sync::load_account_and_credential(state.inner(), &account_id).await {
                Ok(pair) => pair,
                Err(error) => {
                    results.push(serde_json::json!({
                        "account_id": account_id,
                        "status": AccountStatus::Error.to_string(),
                        "message": error.to_string(),
                    }));
                    continue;
                }
            };
        let outcome = status_sync::evaluate_account_refresh(&account, &credential).await?;

        {
            let db = state.db.lock().await;
            let repo = AccountRepository::new(&db);
            status_sync::persist_refresh_outcome(&repo, &account.id, &outcome)?;
        }

        results.push(serde_json::json!({
            "account_id": account.id,
            "status": outcome.status.to_string(),
            "message": outcome.message,
        }));
    }

    Ok(serde_json::to_value(results)?)
}
