use serde_json::Value;
use tauri::State;

use crate::account::{AccountRepository, AccountStatus};
use crate::auth::{AuthService, AuthStatus};
use crate::error::AppError;
use crate::AppState;

#[tauri::command]
pub async fn check_status(
    state: State<'_, AppState>,
    account_id: String,
) -> Result<Value, AppError> {
    let (account, credential) = {
        let db = state.db.lock().await;
        let repo = AccountRepository::new(&db);
        (
            repo.get_by_id(&account_id)?,
            repo.get_credential(&account_id)?,
        )
    };

    let auth_service = AuthService::new();
    let result = auth_service
        .validate_credential(&account, &credential)
        .await?;

    let new_status = match result.status {
        AuthStatus::Valid => AccountStatus::Normal,
        AuthStatus::Expired => AccountStatus::Expired,
        AuthStatus::Invalid => AccountStatus::Error,
        AuthStatus::Unknown => AccountStatus::Unknown,
    };

    {
        let db = state.db.lock().await;
        let repo = AccountRepository::new(&db);
        repo.update_status(&account_id, &new_status, result.message.as_deref())?;
    }

    Ok(serde_json::json!({
        "account_id": account_id,
        "status": new_status.to_string(),
        "message": result.message,
    }))
}

#[tauri::command]
pub async fn check_all_status(state: State<'_, AppState>) -> Result<Value, AppError> {
    let accounts_with_credentials = {
        let db = state.db.lock().await;
        let repo = AccountRepository::new(&db);
        let accounts = repo.list_all()?;
        let mut pairs = Vec::new();

        for account in accounts {
            if let Ok(credential) = repo.get_credential(&account.id) {
                pairs.push((account, credential));
            }
        }

        pairs
    };

    let auth_service = AuthService::new();
    let mut results = Vec::new();

    for (account, credential) in accounts_with_credentials {
        let result = auth_service
            .validate_credential(&account, &credential)
            .await?;
        let new_status = match result.status {
            AuthStatus::Valid => AccountStatus::Normal,
            AuthStatus::Expired => AccountStatus::Expired,
            AuthStatus::Invalid => AccountStatus::Error,
            AuthStatus::Unknown => AccountStatus::Unknown,
        };

        {
            let db = state.db.lock().await;
            let repo = AccountRepository::new(&db);
            repo.update_status(&account.id, &new_status, result.message.as_deref())?;
        }

        results.push(serde_json::json!({
            "account_id": account.id,
            "status": new_status.to_string(),
            "message": result.message,
        }));
    }

    Ok(serde_json::to_value(results)?)
}
