use serde_json::Value;
use tauri::State;

use crate::account::{AccountRepository, CreateAccountInput, UpdateAccountInput};
use crate::error::AppError;
use crate::security;
use crate::AppState;

#[tauri::command]
pub async fn create_account(
    state: State<'_, AppState>,
    input: CreateAccountInput,
) -> Result<Value, AppError> {
    let db = state.db.lock().await;
    let repo = AccountRepository::new(&db);
    let account = repo.create(input)?;
    // Seed demo data
    let usage_repo = crate::usage::UsageRepository::new(&db);
    let _ = usage_repo.seed_demo_data(&account.id);
    Ok(serde_json::to_value(account)?)
}

#[tauri::command]
pub async fn update_account(
    state: State<'_, AppState>,
    input: UpdateAccountInput,
) -> Result<Value, AppError> {
    let db = state.db.lock().await;
    let repo = AccountRepository::new(&db);
    let account = repo.update(input)?;
    Ok(serde_json::to_value(account)?)
}

#[tauri::command]
pub async fn delete_account(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), AppError> {
    let db = state.db.lock().await;
    let repo = AccountRepository::new(&db);
    repo.delete(&id)
}

#[tauri::command]
pub async fn list_accounts(state: State<'_, AppState>) -> Result<Value, AppError> {
    let db = state.db.lock().await;
    let repo = AccountRepository::new(&db);
    let accounts = repo.list_all()?;
    Ok(serde_json::to_value(accounts)?)
}

#[tauri::command]
pub async fn get_account(
    state: State<'_, AppState>,
    id: String,
) -> Result<Value, AppError> {
    let db = state.db.lock().await;
    let repo = AccountRepository::new(&db);
    let account = repo.get_by_id(&id)?;
    Ok(serde_json::to_value(account)?)
}

#[tauri::command]
pub async fn switch_account(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), AppError> {
    let db = state.db.lock().await;
    let repo = AccountRepository::new(&db);
    repo.set_default(&id)?;
    Ok(())
}

#[tauri::command]
pub async fn set_default_account(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), AppError> {
    let db = state.db.lock().await;
    let repo = AccountRepository::new(&db);
    repo.set_default(&id)
}

#[tauri::command]
pub async fn export_accounts(
    state: State<'_, AppState>,
    password: String,
) -> Result<String, AppError> {
    let db = state.db.lock().await;
    let repo = AccountRepository::new(&db);
    let accounts = repo.list_all()?;

    // Build export including credentials
    let mut export_data = Vec::new();
    for account in &accounts {
        let cred = repo.get_credential(&account.id).unwrap_or_default();
        export_data.push(serde_json::json!({
            "account": account,
            "credential": cred,
        }));
    }

    let json = serde_json::to_string(&export_data)?;
    let encrypted = security::encrypt_export(&json, &password)?;
    Ok(encrypted)
}

#[tauri::command]
pub async fn import_accounts(
    state: State<'_, AppState>,
    encrypted_data: String,
    password: String,
) -> Result<usize, AppError> {
    let json = security::decrypt_export(&encrypted_data, &password)?;
    let data: Vec<Value> = serde_json::from_str(&json)?;

    let db = state.db.lock().await;
    let repo = AccountRepository::new(&db);
    let mut count = 0;

    for item in data {
        let account_val = &item["account"];
        let cred = item["credential"].as_str().unwrap_or("").to_string();

        let auth_type = account_val["auth_type"]["__variant"]
            .as_str()
            .unwrap_or("api_key")
            .to_string();

        let input = CreateAccountInput {
            name: account_val["name"].as_str().unwrap_or("Imported").to_string(),
            auth_type,
            email: account_val["email"].as_str().map(String::from),
            organization: account_val["organization"].as_str().map(String::from),
            color: account_val["color"].as_str().map(String::from),
            credential_value: cred,
            credential_type: None,
        };

        if repo.create(input).is_ok() {
            count += 1;
        }
    }

    Ok(count)
}
