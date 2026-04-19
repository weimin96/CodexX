use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::Value;
use tauri::State;
use zip::write::SimpleFileOptions;

use crate::account::{
    Account, AccountRepository, AuthType, CreateAccountInput, UpdateAccountInput,
};
use crate::auth;
use crate::error::AppError;
use crate::local_sync::LocalAuthSyncService;
use crate::AppState;

#[derive(Debug, Serialize)]
pub struct AccountExportResult {
    pub exported_count: usize,
    pub failed_count: usize,
    pub output_path: String,
    pub errors: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct AccountImportResult {
    pub imported_count: usize,
    pub skipped_count: usize,
    pub failed_count: usize,
    pub account_ids: Vec<String>,
    pub errors: Vec<String>,
}

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
    LocalAuthSyncService::write_account_to_default_auth_file(&repo, &id)?;
    Ok(())
}

#[tauri::command]
pub async fn set_default_account(state: State<'_, AppState>, id: String) -> Result<(), AppError> {
    let db = state.db.lock().await;
    let repo = AccountRepository::new(&db);
    repo.set_default(&id)
}

#[tauri::command]
pub async fn export_account_auth_file(
    state: State<'_, AppState>,
    account_id: String,
    output_path: String,
) -> Result<AccountExportResult, AppError> {
    let db = state.db.lock().await;
    let repo = AccountRepository::new(&db);
    let (_, auth_document) =
        LocalAuthSyncService::build_auth_json_for_existing_account(&repo, &account_id)?;
    write_json_file(&output_path, &auth_document)?;

    Ok(AccountExportResult {
        exported_count: 1,
        failed_count: 0,
        output_path,
        errors: Vec::new(),
    })
}

#[tauri::command]
pub async fn export_accounts(
    state: State<'_, AppState>,
    output_path: String,
) -> Result<AccountExportResult, AppError> {
    let db = state.db.lock().await;
    let repo = AccountRepository::new(&db);
    let accounts = repo.list_all()?;
    let output = non_empty_path(&output_path, "请选择 zip 导出路径")?;
    ensure_parent_dir(output)?;

    let file = File::create(output)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o600);
    let mut exported_count = 0;
    let mut errors = Vec::new();

    for account in accounts {
        match LocalAuthSyncService::build_auth_json_for_existing_account(&repo, &account.id) {
            Ok((account, auth_document)) => {
                let entry_name = build_auth_json_entry_name(&account);
                let content = serde_json::to_vec_pretty(&auth_document)?;
                zip.start_file(entry_name, options)
                    .map_err(|error| AppError::Other(format!("写入 zip 条目失败: {error}")))?;
                zip.write_all(&content)?;
                exported_count += 1;
            }
            Err(error) => {
                errors.push(format!("{}: {}", account_email_or_name(&account), error));
            }
        }
    }

    zip.finish()
        .map_err(|error| AppError::Other(format!("完成 zip 写入失败: {error}")))?;

    if exported_count == 0 {
        return Err(AppError::InvalidInput(
            "没有可导出的标准 auth.json 账号".to_string(),
        ));
    }

    Ok(AccountExportResult {
        exported_count,
        failed_count: errors.len(),
        output_path,
        errors,
    })
}

#[tauri::command]
pub async fn import_accounts(
    state: State<'_, AppState>,
    input_path: String,
) -> Result<AccountImportResult, AppError> {
    let db = state.db.lock().await;
    let repo = AccountRepository::new(&db);
    let path = non_empty_path(&input_path, "请选择要导入的 auth.json 或 zip 文件")?;

    match path.extension().and_then(|extension| extension.to_str()) {
        Some(extension) if extension.eq_ignore_ascii_case("zip") => {
            import_accounts_from_zip(&repo, path)
        }
        Some(extension) if extension.eq_ignore_ascii_case("json") => {
            import_account_from_json_file(&repo, path)
        }
        _ => Err(AppError::InvalidInput(
            "导入文件只支持 auth.json 或 zip 压缩包".to_string(),
        )),
    }
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

#[tauri::command]
pub async fn sync_local_default_account(state: State<'_, AppState>) -> Result<Value, AppError> {
    let prepared = match LocalAuthSyncService::prepare_auth_file(None) {
        Ok(prepared) => prepared,
        Err(error) => {
            return Ok(serde_json::json!({
                "matched_account_id": null,
                "updated": false,
                "skipped_reason": error.to_string(),
            }));
        }
    };

    let result = {
        let db = state.db.lock().await;
        let repo = AccountRepository::new(&db);
        LocalAuthSyncService::sync_default_account_marker(&repo, &prepared)?
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

fn import_account_from_json_file(
    repo: &AccountRepository<'_>,
    path: &Path,
) -> Result<AccountImportResult, AppError> {
    let raw_text = std::fs::read_to_string(path)?;
    let result = import_auth_json_text(repo, path.to_path_buf(), &raw_text)?;

    Ok(AccountImportResult {
        imported_count: 1,
        skipped_count: 0,
        failed_count: 0,
        account_ids: vec![result.account_id],
        errors: Vec::new(),
    })
}

fn import_accounts_from_zip(
    repo: &AccountRepository<'_>,
    path: &Path,
) -> Result<AccountImportResult, AppError> {
    let file = File::open(path)?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|error| AppError::InvalidInput(format!("读取 zip 压缩包失败: {error}")))?;
    let mut imported_count = 0;
    let mut skipped_count = 0;
    let mut account_ids = Vec::new();
    let mut errors = Vec::new();

    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|error| AppError::InvalidInput(format!("读取 zip 条目失败: {error}")))?;
        let entry_name = entry.name().replace('\\', "/");

        if entry.is_dir() || !entry_name.to_lowercase().ends_with(".json") {
            skipped_count += 1;
            continue;
        }

        let mut raw_text = String::new();
        match entry.read_to_string(&mut raw_text) {
            Ok(_) => {
                let source_path = PathBuf::from(format!("{}::{entry_name}", path.display()));
                match import_auth_json_text(repo, source_path, &raw_text) {
                    Ok(result) => {
                        imported_count += 1;
                        account_ids.push(result.account_id);
                    }
                    Err(error) => {
                        errors.push(format!("{entry_name}: {error}"));
                    }
                }
            }
            Err(error) => {
                errors.push(format!("{entry_name}: 读取 JSON 失败: {error}"));
            }
        }
    }

    if imported_count == 0 {
        if errors.is_empty() {
            return Err(AppError::InvalidInput(
                "zip 中没有可导入的 auth.json 文件".to_string(),
            ));
        }

        return Err(AppError::InvalidInput(format!(
            "zip 中没有可成功导入的 auth.json：{}",
            errors.join("；")
        )));
    }

    Ok(AccountImportResult {
        imported_count,
        skipped_count,
        failed_count: errors.len(),
        account_ids,
        errors,
    })
}

fn import_auth_json_text(
    repo: &AccountRepository<'_>,
    source_path: PathBuf,
    raw_text: &str,
) -> Result<crate::local_sync::LocalAuthSyncResult, AppError> {
    let prepared = LocalAuthSyncService::prepare_auth_text(source_path, raw_text)?;
    LocalAuthSyncService::sync_prepared_auth_preserving_profile(repo, prepared)
}

fn write_json_file(output_path: &str, document: &Value) -> Result<(), AppError> {
    let path = non_empty_path(output_path, "请选择 auth.json 导出路径")?;
    ensure_parent_dir(path)?;
    let content = serde_json::to_vec_pretty(document)?;
    std::fs::write(path, content)?;
    Ok(())
}

fn ensure_parent_dir(path: &Path) -> Result<(), AppError> {
    let parent = path.parent().ok_or_else(|| {
        AppError::InvalidInput(format!("无法识别导出目录：{}", path.to_string_lossy()))
    })?;

    if parent.as_os_str().is_empty() {
        return Ok(());
    }

    std::fs::create_dir_all(parent)?;
    Ok(())
}

fn non_empty_path<'a>(path: &'a str, empty_message: &str) -> Result<&'a Path, AppError> {
    let normalized = path.trim();
    if normalized.is_empty() {
        return Err(AppError::InvalidInput(empty_message.to_string()));
    }

    Ok(Path::new(normalized))
}

fn build_auth_json_entry_name(account: &Account) -> String {
    let name = sanitize_file_stem(&account_email_or_name(account));
    let suffix = account
        .id
        .chars()
        .filter(|value| value.is_ascii_alphanumeric())
        .take(8)
        .collect::<String>();

    format!("{name}-auth-{suffix}.json")
}

fn account_email_or_name(account: &Account) -> String {
    account
        .email
        .as_deref()
        .map(str::trim)
        .filter(|email| !email.is_empty())
        .unwrap_or(&account.name)
        .to_string()
}

fn sanitize_file_stem(value: &str) -> String {
    let sanitized = value
        .trim()
        .chars()
        .map(|character| match character {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            character if character.is_control() => '_',
            character => character,
        })
        .collect::<String>();
    let trimmed = sanitized.trim_matches([' ', '.']);

    if trimmed.is_empty() {
        "account".to_string()
    } else {
        trimmed.chars().take(80).collect()
    }
}
