use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::account::{
    Account, AccountRepository, AuthType, CodexAccountProfile, UpsertSyncedAccountInput,
};
use crate::auth;
use crate::codex_usage;
use crate::error::{AppError, AppResult};

#[derive(Debug, Serialize)]
pub struct LocalAuthSyncResult {
    pub account_id: String,
    pub account_name: String,
    pub auth_type: String,
    pub auth_file_path: String,
    pub codex_plan_type: Option<String>,
    pub codex_usage_error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LocalDefaultAccountSyncResult {
    pub matched_account_id: Option<String>,
    pub updated: bool,
    pub skipped_reason: Option<String>,
}

#[derive(Debug)]
pub struct PreparedLocalAuthSync {
    resolved_path: PathBuf,
    stable_id: String,
    legacy_path_stable_id: String,
    name: String,
    auth_type: AuthType,
    email: Option<String>,
    organization: Option<String>,
    credential_value: String,
    credential_type: Option<String>,
    codex_profile_seed: Option<CodexAccountProfile>,
    codex_usage_source: Option<CodexUsageSource>,
}

#[derive(Debug, Clone)]
struct CodexUsageSource {
    access_token: String,
    account_id: String,
}

impl PreparedLocalAuthSync {
    pub(crate) fn stable_id(&self) -> &str {
        &self.stable_id
    }

    pub(crate) fn can_recover_account(&self, account: &Account) -> bool {
        if self.stable_id == account.id {
            return true;
        }

        if self.legacy_path_stable_id != account.id || self.auth_type != account.auth_type {
            return false;
        }

        match self.auth_type {
            AuthType::OAuthToken => self
                .email
                .as_deref()
                .zip(account.email.as_deref())
                .is_some_and(|(current_email, account_email)| {
                    current_email.eq_ignore_ascii_case(account_email)
                }),
            AuthType::ApiKey => false,
            AuthType::CookieSession | AuthType::CliProfile => false,
        }
    }

    pub(crate) fn credential_value(&self) -> &str {
        &self.credential_value
    }
}

#[derive(Debug)]
struct LocalAuthFileDocument {
    raw: Value,
    openai_api_key: Option<String>,
    auth_mode: Option<String>,
    tokens: Option<LocalAuthTokens>,
}

#[derive(Debug, Deserialize)]
struct LocalAuthFileFields {
    #[serde(rename = "OPENAI_API_KEY")]
    openai_api_key: Option<String>,
    auth_mode: Option<String>,
    tokens: Option<LocalAuthTokens>,
}

#[derive(Debug, Clone, Deserialize)]
struct LocalAuthTokens {
    access_token: Option<String>,
    account_id: Option<String>,
    id_token: Option<String>,
}

pub struct LocalAuthSyncService;

impl LocalAuthSyncService {
    pub fn prepare_auth_file(auth_file_path: Option<&str>) -> AppResult<PreparedLocalAuthSync> {
        let resolved_path = Self::resolve_auth_file_path(auth_file_path)?;
        let auth_document = Self::read_auth_document(&resolved_path)?;
        Self::build_prepared_sync(resolved_path, auth_document)
    }

    pub fn prepare_auth_text(
        source_path: PathBuf,
        raw_text: &str,
    ) -> AppResult<PreparedLocalAuthSync> {
        let raw: Value = serde_json::from_str(raw_text)?;
        let fields: LocalAuthFileFields = serde_json::from_value(raw.clone())?;

        Self::build_prepared_sync(
            source_path,
            LocalAuthFileDocument {
                raw,
                openai_api_key: fields.openai_api_key,
                auth_mode: fields.auth_mode,
                tokens: fields.tokens,
            },
        )
    }

    pub fn build_auth_json_for_existing_account(
        repo: &AccountRepository<'_>,
        account_id: &str,
    ) -> AppResult<(Account, Value)> {
        let account = repo.get_by_id(account_id)?;
        let credential = repo.get_credential(account_id)?;
        let auth_document = Self::build_auth_json_for_account(&account, &credential)?;

        Ok((account, auth_document))
    }

    pub async fn fetch_codex_profile(
        prepared: &PreparedLocalAuthSync,
    ) -> Option<CodexAccountProfile> {
        let mut seed = prepared.codex_profile_seed.clone()?;
        let Some(source) = prepared.codex_usage_source.clone() else {
            if seed.fetched_at.is_none() {
                seed.fetched_at = Some(Utc::now().to_rfc3339());
            }
            return Some(seed);
        };

        match codex_usage::fetch_codex_account_profile(
            &source.access_token,
            &source.account_id,
            seed.plan_type.clone(),
        )
        .await
        {
            Ok(profile) => Some(profile),
            Err(error) => {
                seed.fetched_at = Some(Utc::now().to_rfc3339());
                seed.usage_error = Some(error.to_string());
                Some(seed)
            }
        }
    }

    pub fn sync_prepared_auth(
        repo: &AccountRepository<'_>,
        prepared: PreparedLocalAuthSync,
        codex_profile: Option<CodexAccountProfile>,
    ) -> AppResult<LocalAuthSyncResult> {
        Self::sync_prepared_auth_inner(repo, prepared, codex_profile, true)
    }

    pub fn sync_prepared_auth_preserving_profile(
        repo: &AccountRepository<'_>,
        prepared: PreparedLocalAuthSync,
    ) -> AppResult<LocalAuthSyncResult> {
        Self::sync_prepared_auth_inner(repo, prepared, None, false)
    }

    fn sync_prepared_auth_inner(
        repo: &AccountRepository<'_>,
        prepared: PreparedLocalAuthSync,
        codex_profile: Option<CodexAccountProfile>,
        use_profile_seed: bool,
    ) -> AppResult<LocalAuthSyncResult> {
        let merge_source_ids = Self::find_merge_source_ids(repo, &prepared)?;
        let PreparedLocalAuthSync {
            resolved_path,
            stable_id,
            legacy_path_stable_id: _,
            name,
            auth_type,
            email,
            organization,
            credential_value,
            credential_type,
            codex_profile_seed,
            codex_usage_source: _,
        } = prepared;
        let profile = if use_profile_seed {
            codex_profile.or(codex_profile_seed)
        } else {
            codex_profile
        };
        let account = repo.upsert_synced_account(UpsertSyncedAccountInput {
            stable_id,
            name,
            auth_type,
            email,
            organization,
            color: Some("#4f8ef7".to_string()),
            credential_value,
            credential_type,
            codex_profile: profile,
        })?;
        for source_id in merge_source_ids {
            if source_id != account.id {
                repo.merge_accounts(&account.id, &source_id)?;
            }
        }
        let account = repo.get_by_id(&account.id)?;
        let account_name = account_email_or_name(&account);

        Ok(LocalAuthSyncResult {
            account_id: account.id,
            account_name,
            auth_type: account.auth_type.to_string(),
            auth_file_path: resolved_path.to_string_lossy().to_string(),
            codex_plan_type: account.codex_plan_type,
            codex_usage_error: account.codex_usage_error,
        })
    }

    pub fn sync_default_account_marker(
        repo: &AccountRepository<'_>,
        prepared: &PreparedLocalAuthSync,
    ) -> AppResult<LocalDefaultAccountSyncResult> {
        let accounts = repo.list_all()?;
        let matched_account_id = Self::find_matching_account_id(repo, prepared, &accounts)?;

        if let Some(account_id) = matched_account_id {
            let matched_account = accounts.iter().find(|account| account.id == account_id);
            let was_default = matched_account.is_some_and(|account| account.is_default);
            if !was_default {
                repo.set_default(&account_id)?;
            }

            return Ok(LocalDefaultAccountSyncResult {
                matched_account_id: Some(account_id),
                updated: !was_default,
                skipped_reason: None,
            });
        }

        let had_default = accounts.iter().any(|account| account.is_default);
        if had_default {
            repo.clear_default()?;
        }

        Ok(LocalDefaultAccountSyncResult {
            matched_account_id: None,
            updated: had_default,
            skipped_reason: Some("当前 auth.json 对应账号尚未导入".to_string()),
        })
    }

    pub fn write_account_to_default_auth_file(
        repo: &AccountRepository<'_>,
        account_id: &str,
    ) -> AppResult<Account> {
        let account = repo.get_by_id(account_id)?;
        let credential = repo.get_credential(account_id)?;
        let auth_document = Self::build_auth_json_for_account(&account, &credential)?;
        let auth_file_path = Self::resolve_auth_file_path(None)?;

        Self::write_auth_document_with_backup(&auth_file_path, &auth_document)?;
        repo.set_default(account_id)?;
        repo.get_by_id(account_id)
    }

    fn find_merge_source_ids(
        repo: &AccountRepository<'_>,
        prepared: &PreparedLocalAuthSync,
    ) -> AppResult<Vec<String>> {
        let mut candidate_ids = Vec::new();

        for account in repo.list_all()? {
            if !account.id.starts_with("local-auth-") || account.auth_type != prepared.auth_type {
                continue;
            }

            if account.id == prepared.stable_id {
                candidate_ids.push(account.id);
                continue;
            }

            if Self::is_same_local_sync_identity(repo, &account, prepared)? {
                candidate_ids.push(account.id);
            }
        }

        candidate_ids.sort();
        candidate_ids.dedup();
        Ok(candidate_ids)
    }

    fn find_matching_account_id(
        repo: &AccountRepository<'_>,
        prepared: &PreparedLocalAuthSync,
        accounts: &[Account],
    ) -> AppResult<Option<String>> {
        if let Some(account) = accounts
            .iter()
            .find(|account| account.id == prepared.stable_id)
        {
            return Ok(Some(account.id.clone()));
        }

        for account in accounts {
            if account.auth_type != prepared.auth_type {
                continue;
            }

            if prepared.can_recover_account(account)
                || Self::is_same_local_sync_identity(repo, account, prepared)?
            {
                return Ok(Some(account.id.clone()));
            }
        }

        Ok(None)
    }

    fn resolve_auth_file_path(auth_file_path: Option<&str>) -> AppResult<PathBuf> {
        if let Some(path) = Self::non_empty_text(auth_file_path) {
            return Ok(PathBuf::from(path));
        }

        if let Ok(codex_home) = std::env::var("CODEX_HOME") {
            if let Some(path) = Self::non_empty_text(Some(codex_home.as_str())) {
                return Ok(PathBuf::from(path).join("auth.json"));
            }
        }

        if let Ok(user_profile) = std::env::var("USERPROFILE") {
            if let Some(path) = Self::non_empty_text(Some(user_profile.as_str())) {
                return Ok(PathBuf::from(path).join(".codex").join("auth.json"));
            }
        }

        if let Ok(home) = std::env::var("HOME") {
            if let Some(path) = Self::non_empty_text(Some(home.as_str())) {
                return Ok(PathBuf::from(path).join(".codex").join("auth.json"));
            }
        }

        Err(AppError::InvalidInput(
            "无法推断本地 auth.json 路径，请确认 CODEX_HOME 或 USERPROFILE 环境变量可用"
                .to_string(),
        ))
    }

    fn build_auth_json_for_account(account: &Account, credential: &str) -> AppResult<Value> {
        match account.auth_type {
            AuthType::OAuthToken => {
                let auth_json: Value = serde_json::from_str(credential).map_err(|_| {
                    AppError::InvalidInput(
                        "该 OAuth 账号缺少完整 auth.json，无法切换本地默认账号".to_string(),
                    )
                })?;
                let has_access_token = auth_json
                    .get("tokens")
                    .and_then(Value::as_object)
                    .and_then(|tokens| tokens.get("access_token"))
                    .or_else(|| auth_json.get("access_token"))
                    .and_then(Value::as_str)
                    .and_then(|value| Self::non_empty_text(Some(value)))
                    .is_some();

                if !has_access_token {
                    return Err(AppError::InvalidInput(
                        "该 OAuth 账号没有可写回的访问令牌，无法切换本地默认账号".to_string(),
                    ));
                }

                Ok(auth_json)
            }
            AuthType::ApiKey => {
                let api_key = Self::non_empty_text(Some(credential)).ok_or_else(|| {
                    AppError::InvalidInput(
                        "该 API Key 账号凭证为空，无法切换本地默认账号".to_string(),
                    )
                })?;

                Ok(serde_json::json!({
                    "OPENAI_API_KEY": api_key,
                    "auth_mode": "apikey",
                    "last_refresh": Utc::now().to_rfc3339(),
                }))
            }
            AuthType::CookieSession | AuthType::CliProfile => Err(AppError::InvalidInput(
                "该账号类型不能写回 Codex auth.json".to_string(),
            )),
        }
    }

    fn write_auth_document_with_backup(path: &Path, document: &Value) -> AppResult<()> {
        let parent = path.parent().ok_or_else(|| {
            AppError::InvalidInput(format!(
                "无法识别 auth.json 所在目录：{}",
                path.to_string_lossy()
            ))
        })?;
        std::fs::create_dir_all(parent)?;

        let backup_path = path.with_file_name("auth.json.bak");
        let temp_path = path.with_file_name(format!("auth.json.tmp-{}", std::process::id()));
        let auth_text = serde_json::to_string_pretty(document)?;

        std::fs::write(&temp_path, auth_text.as_bytes())?;
        if path.exists() {
            std::fs::copy(path, &backup_path)?;
            std::fs::remove_file(path)?;
        }

        if let Err(error) = std::fs::rename(&temp_path, path) {
            if backup_path.exists() {
                let _ = std::fs::copy(&backup_path, path);
            }
            let _ = std::fs::remove_file(&temp_path);
            return Err(error.into());
        }

        Ok(())
    }

    fn read_auth_document(path: &Path) -> AppResult<LocalAuthFileDocument> {
        if !path.exists() {
            return Err(AppError::InvalidInput(format!(
                "未找到 auth.json 文件：{}",
                path.to_string_lossy()
            )));
        }

        let raw_text = std::fs::read_to_string(path)?;
        let raw: Value = serde_json::from_str(&raw_text)?;
        let fields: LocalAuthFileFields = serde_json::from_value(raw.clone())?;

        Ok(LocalAuthFileDocument {
            raw,
            openai_api_key: fields.openai_api_key,
            auth_mode: fields.auth_mode,
            tokens: fields.tokens,
        })
    }

    fn build_prepared_sync(
        resolved_path: PathBuf,
        auth_document: LocalAuthFileDocument,
    ) -> AppResult<PreparedLocalAuthSync> {
        let legacy_path_stable_id = Self::build_path_stable_account_id(&resolved_path);
        let source_path = Self::normalize_path(&resolved_path);

        if let Some(api_key) = Self::non_empty_text(auth_document.openai_api_key.as_deref()) {
            return Ok(PreparedLocalAuthSync {
                resolved_path,
                stable_id: Self::build_identity_stable_account_id("api-key", api_key),
                legacy_path_stable_id,
                name: "API Key 账号".to_string(),
                auth_type: AuthType::ApiKey,
                email: None,
                organization: Some("本地文件同步".to_string()),
                credential_value: api_key.to_string(),
                credential_type: None,
                codex_profile_seed: None,
                codex_usage_source: None,
            });
        }

        let Some(tokens) = auth_document.tokens.as_ref() else {
            return Err(Self::missing_credential_error(
                &source_path,
                auth_document.auth_mode.as_deref(),
            ));
        };
        let Some(access_token) = Self::non_empty_text(tokens.access_token.as_deref()) else {
            return Err(Self::missing_credential_error(
                &source_path,
                auth_document.auth_mode.as_deref(),
            ));
        };

        let identity = Self::extract_oauth_identity(tokens);
        let organization = Some("本地文件同步".to_string());
        let usage_error = if identity.account_id.is_none() {
            Some("auth.json 缺少 account_id，无法请求 Codex 用量接口".to_string())
        } else {
            None
        };
        let profile_seed = Some(CodexAccountProfile {
            plan_type: identity.plan_type.clone(),
            fetched_at: None,
            five_hour: None,
            one_week: None,
            usage_error,
        });
        let stable_id = identity
            .account_id
            .as_deref()
            .map(|account_id| Self::build_identity_stable_account_id("chatgpt", account_id))
            .unwrap_or_else(|| legacy_path_stable_id.clone());
        let usage_source = identity
            .account_id
            .as_ref()
            .map(|account_id| CodexUsageSource {
                access_token: access_token.to_string(),
                account_id: account_id.clone(),
            });
        let credential_value = serde_json::to_string(&auth_document.raw)?;

        Ok(PreparedLocalAuthSync {
            resolved_path,
            stable_id,
            legacy_path_stable_id,
            name: auth::resolve_account_display_name(
                identity.name.as_deref(),
                identity.email.as_deref(),
                "OAuth 账号",
            ),
            auth_type: AuthType::OAuthToken,
            email: identity.email,
            organization,
            credential_value,
            credential_type: Some("oauth_json".to_string()),
            codex_profile_seed: profile_seed,
            codex_usage_source: usage_source,
        })
    }

    fn is_same_local_sync_identity(
        repo: &AccountRepository<'_>,
        account: &Account,
        prepared: &PreparedLocalAuthSync,
    ) -> AppResult<bool> {
        if account.id == prepared.legacy_path_stable_id {
            if let Ok(credential) = repo.get_credential(&account.id) {
                if Self::extract_identity_stable_id_from_credential(
                    &prepared.auth_type,
                    &credential,
                )
                .as_deref()
                    == Some(prepared.stable_id.as_str())
                {
                    return Ok(true);
                }
            }

            return Ok(Self::emails_match(
                account.email.as_deref(),
                prepared.email.as_deref(),
            ));
        }

        let credential = match repo.get_credential(&account.id) {
            Ok(credential) => credential,
            Err(_) => return Ok(false),
        };

        Ok(
            Self::extract_identity_stable_id_from_credential(&prepared.auth_type, &credential)
                .as_deref()
                == Some(prepared.stable_id.as_str()),
        )
    }

    fn extract_identity_stable_id_from_credential(
        auth_type: &AuthType,
        credential: &str,
    ) -> Option<String> {
        match auth_type {
            AuthType::ApiKey => {
                let api_key = Self::non_empty_text(Some(credential))?;
                Some(Self::build_identity_stable_account_id("api-key", api_key))
            }
            AuthType::OAuthToken => {
                let credential_json = serde_json::from_str::<Value>(credential).ok()?;
                let tokens = credential_json.get("tokens").and_then(Value::as_object);
                let claims = tokens
                    .and_then(|tokens| tokens.get("id_token"))
                    .or_else(|| credential_json.get("id_token"))
                    .and_then(Value::as_str)
                    .and_then(|token| auth::decode_jwt_payload(token).ok());
                let auth_claim = claims
                    .as_ref()
                    .and_then(|value| value.get("https://api.openai.com/auth"))
                    .and_then(Value::as_object);
                let account_id = tokens
                    .and_then(|tokens| tokens.get("account_id"))
                    .or_else(|| credential_json.get("account_id"))
                    .and_then(Value::as_str)
                    .and_then(|value| Self::non_empty_text(Some(value)))
                    .map(ToString::to_string)
                    .or_else(|| {
                        auth_claim
                            .and_then(|value| value.get("chatgpt_account_id"))
                            .and_then(Value::as_str)
                            .and_then(|value| Self::non_empty_text(Some(value)))
                            .map(ToString::to_string)
                    })?;

                Some(Self::build_identity_stable_account_id(
                    "chatgpt",
                    &account_id,
                ))
            }
            AuthType::CookieSession | AuthType::CliProfile => None,
        }
    }

    fn extract_oauth_identity(tokens: &LocalAuthTokens) -> LocalOAuthIdentity {
        let claims = tokens
            .id_token
            .as_deref()
            .and_then(|id_token| auth::decode_jwt_payload(id_token).ok());
        let auth_claim = claims
            .as_ref()
            .and_then(|value| value.get("https://api.openai.com/auth"))
            .and_then(Value::as_object);
        let account_id = tokens
            .account_id
            .as_deref()
            .and_then(|value| Self::non_empty_text(Some(value)))
            .map(ToString::to_string)
            .or_else(|| {
                auth_claim
                    .and_then(|value| value.get("chatgpt_account_id"))
                    .and_then(Value::as_str)
                    .and_then(|value| Self::non_empty_text(Some(value)))
                    .map(ToString::to_string)
            });
        let email = claims
            .as_ref()
            .and_then(|value| value.get("email"))
            .and_then(Value::as_str)
            .and_then(|value| Self::non_empty_text(Some(value)))
            .map(ToString::to_string);
        let name = claims
            .as_ref()
            .and_then(|value| value.get("name"))
            .and_then(Value::as_str)
            .and_then(|value| Self::non_empty_text(Some(value)))
            .map(ToString::to_string);
        let plan_type = auth_claim
            .and_then(|value| value.get("chatgpt_plan_type"))
            .and_then(Value::as_str)
            .and_then(|value| Self::non_empty_text(Some(value)))
            .map(ToString::to_string);

        LocalOAuthIdentity {
            account_id,
            name,
            email,
            plan_type,
        }
    }

    fn missing_credential_error(path: &Path, auth_mode: Option<&str>) -> AppError {
        AppError::InvalidInput(format!(
            "auth.json 中未找到可同步的凭证字段：{}，当前模式 {:?}",
            path.to_string_lossy(),
            auth_mode
        ))
    }

    fn build_path_stable_account_id(path: &Path) -> String {
        let normalized = Self::normalize_path(path);
        let normalized_text = normalized
            .to_string_lossy()
            .replace('/', "\\")
            .to_lowercase();
        let suffix = Self::digest_suffix("path", &normalized_text);

        format!("local-auth-{}", suffix)
    }

    fn build_identity_stable_account_id(identity_kind: &str, identity_value: &str) -> String {
        let normalized_value = identity_value.trim();
        let suffix = Self::digest_suffix(identity_kind, normalized_value);

        format!("local-auth-{identity_kind}-{suffix}")
    }

    fn digest_suffix(scope: &str, value: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(scope.as_bytes());
        hasher.update([0]);
        hasher.update(value.as_bytes());
        let digest = hasher.finalize();
        let mut suffix = String::with_capacity(24);

        for byte in digest.iter().take(12) {
            let _ = write!(&mut suffix, "{:02x}", byte);
        }
        suffix
    }

    fn normalize_path(path: &Path) -> PathBuf {
        path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
    }

    fn emails_match(left: Option<&str>, right: Option<&str>) -> bool {
        match (left, right) {
            (Some(left), Some(right)) => left.trim().eq_ignore_ascii_case(right.trim()),
            _ => false,
        }
    }

    fn non_empty_text(value: Option<&str>) -> Option<&str> {
        value.map(str::trim).filter(|text| !text.is_empty())
    }
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

struct LocalOAuthIdentity {
    account_id: Option<String>,
    name: Option<String>,
    email: Option<String>,
    plan_type: Option<String>,
}
