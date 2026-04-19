use serde_json::Value;
use tauri::State;

use crate::account::{AccountRepository, AuthType, CreateAccountInput, UpdateAccountInput};
use crate::auth;
use crate::error::AppError;
use crate::local_sync::LocalAuthSyncService;
use crate::security;
use crate::AppState;

#[tauri::command]
pub async fn create_account(
    state: State<'_, AppState>,
    input: CreateAccountInput,
) -> Result<Value, AppError> {
    let resolved_input = resolve_create_account_input(input).await?;
    let db = state.db.lock().await;
    let repo = AccountRepository::new(&db);
    let account = repo.create(resolved_input)?;
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
pub async fn delete_account(state: State<'_, AppState>, id: String) -> Result<(), AppError> {
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
pub async fn get_account(state: State<'_, AppState>, id: String) -> Result<Value, AppError> {
    let db = state.db.lock().await;
    let repo = AccountRepository::new(&db);
    let account = repo.get_by_id(&id)?;
    Ok(serde_json::to_value(account)?)
}

#[tauri::command]
pub async fn get_account_credential(
    state: State<'_, AppState>,
    id: String,
) -> Result<String, AppError> {
    let db = state.db.lock().await;
    let repo = AccountRepository::new(&db);
    repo.get_credential(&id)
}

#[tauri::command]
pub async fn switch_account(state: State<'_, AppState>, id: String) -> Result<(), AppError> {
    let db = state.db.lock().await;
    let repo = AccountRepository::new(&db);
    repo.set_default(&id)?;
    Ok(())
}

#[tauri::command]
pub async fn set_default_account(state: State<'_, AppState>, id: String) -> Result<(), AppError> {
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

    // 导出必须包含凭证密文解密后的明文，再由用户密码整体加密。
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
            name: account_val["name"].as_str().map(String::from),
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

#[tauri::command]
pub async fn sync_local_auth_file(
    state: State<'_, AppState>,
    auth_file_path: Option<String>,
) -> Result<Value, AppError> {
    let prepared = LocalAuthSyncService::prepare_auth_file(auth_file_path.as_deref())?;
    let codex_profile = LocalAuthSyncService::fetch_codex_profile(&prepared).await;
    let result = {
        let db = state.db.lock().await;
        let repo = AccountRepository::new(&db);
        LocalAuthSyncService::sync_prepared_auth(&repo, prepared, codex_profile)?
    };
    Ok(serde_json::to_value(result)?)
}

async fn resolve_create_account_input(
    input: CreateAccountInput,
) -> Result<CreateAccountInput, AppError> {
    let CreateAccountInput {
        name,
        auth_type,
        email,
        organization,
        color,
        credential_value,
        credential_type,
    } = input;
    let auth_type = AuthType::try_from(auth_type.as_str())?;
    let mut resolved_name = normalize_optional_text(name);
    let mut resolved_email = normalize_optional_text(email);
    let mut resolved_organization = normalize_optional_text(organization);

    // 新增账号不再要求用户手填名称，优先使用凭证可解析出的身份信息，避免账号列表混入临时占位名。
    if resolved_name.is_none() || resolved_email.is_none() || resolved_organization.is_none() {
        if let Ok(identity) = auth::resolve_credential_identity(&auth_type, &credential_value).await
        {
            if resolved_name.is_none() {
                resolved_name = identity.name;
            }
            if resolved_email.is_none() {
                resolved_email = identity.email;
            }
            if resolved_organization.is_none() {
                resolved_organization = identity.organization;
            }
        }
    }

    let fallback_name = fallback_account_name(&auth_type);
    Ok(CreateAccountInput {
        name: Some(auth::resolve_account_display_name(
            resolved_name.as_deref(),
            resolved_email.as_deref(),
            fallback_name,
        )),
        auth_type: auth_type.to_string(),
        email: resolved_email,
        organization: resolved_organization,
        color,
        credential_value,
        credential_type,
    })
}

fn normalize_optional_text(value: Option<String>) -> Option<String> {
    value.and_then(|text| {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn fallback_account_name(auth_type: &AuthType) -> &'static str {
    match auth_type {
        AuthType::ApiKey => "API Key 账号",
        AuthType::OAuthToken => "OAuth 账号",
        AuthType::CookieSession => "Session 账号",
        AuthType::CliProfile => "CLI 账号",
    }
}
