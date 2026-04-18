use serde_json::Value;
use tauri::State;

use crate::account::AccountRepository;
use crate::auth::AuthService;
use crate::error::AppError;
use crate::AppState;

#[tauri::command]
pub async fn refresh_token(
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
    Ok(serde_json::to_value(result)?)
}

#[tauri::command]
pub async fn validate_token(
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
    Ok(serde_json::to_value(result)?)
}

#[tauri::command]
pub async fn get_auth_status(
    state: State<'_, AppState>,
    account_id: String,
) -> Result<Value, AppError> {
    let db = state.db.lock().await;
    let repo = AccountRepository::new(&db);
    let account = repo.get_by_id(&account_id)?;
    Ok(serde_json::to_value(&account.status)?)
}
