use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::account::{AccountRepository, AuthType, CodexAccountProfile, UpsertSyncedAccountInput};
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

#[derive(Debug)]
pub struct PreparedLocalAuthSync {
    resolved_path: PathBuf,
    stable_id: String,
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
        let PreparedLocalAuthSync {
            resolved_path,
            stable_id,
            name,
            auth_type,
            email,
            organization,
            credential_value,
            credential_type,
            codex_profile_seed,
            codex_usage_source: _,
        } = prepared;
        let profile = codex_profile.or(codex_profile_seed);
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

        Ok(LocalAuthSyncResult {
            account_id: account.id,
            account_name: account.name,
            auth_type: account.auth_type.to_string(),
            auth_file_path: resolved_path.to_string_lossy().to_string(),
            codex_plan_type: account.codex_plan_type,
            codex_usage_error: account.codex_usage_error,
        })
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
        let stable_id = Self::build_stable_account_id(&resolved_path);
        let source_path = Self::normalize_path(&resolved_path);

        if let Some(api_key) = Self::non_empty_text(auth_document.openai_api_key.as_deref()) {
            return Ok(PreparedLocalAuthSync {
                resolved_path,
                stable_id,
                name: "本地 auth.json（API Key）".to_string(),
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
        let account_hint = identity
            .account_id
            .as_deref()
            .map(Self::shorten_account_hint);
        let organization = match account_hint {
            Some(account_id) => Some(format!("本地文件同步 · {}", account_id)),
            None => Some("本地文件同步".to_string()),
        };
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
        let usage_source = identity.account_id.map(|account_id| CodexUsageSource {
            access_token: access_token.to_string(),
            account_id,
        });
        let credential_value = serde_json::to_string(&auth_document.raw)?;

        Ok(PreparedLocalAuthSync {
            resolved_path,
            stable_id,
            name: identity
                .email
                .as_ref()
                .map(|email| format!("本地 auth.json（{}）", email))
                .unwrap_or_else(|| "本地 auth.json（OAuth）".to_string()),
            auth_type: AuthType::OAuthToken,
            email: identity.email,
            organization,
            credential_value,
            credential_type: Some("oauth_json".to_string()),
            codex_profile_seed: profile_seed,
            codex_usage_source: usage_source,
        })
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
        let plan_type = auth_claim
            .and_then(|value| value.get("chatgpt_plan_type"))
            .and_then(Value::as_str)
            .and_then(|value| Self::non_empty_text(Some(value)))
            .map(ToString::to_string);

        LocalOAuthIdentity {
            account_id,
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

    fn build_stable_account_id(path: &Path) -> String {
        let normalized = Self::normalize_path(path);
        let normalized_text = normalized
            .to_string_lossy()
            .replace('/', "\\")
            .to_lowercase();
        let digest = Sha256::digest(normalized_text.as_bytes());
        let mut suffix = String::with_capacity(24);

        for byte in digest.iter().take(12) {
            let _ = write!(&mut suffix, "{:02x}", byte);
        }

        format!("local-auth-{}", suffix)
    }

    fn normalize_path(path: &Path) -> PathBuf {
        path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
    }

    fn non_empty_text<'a>(value: Option<&'a str>) -> Option<&'a str> {
        value.map(str::trim).filter(|text| !text.is_empty())
    }

    fn shorten_account_hint(account_id: &str) -> String {
        let mut chars = account_id.chars();
        let prefix: String = chars.by_ref().take(6).collect();
        let remaining = chars.count();

        if remaining == 0 {
            prefix
        } else {
            format!("{}...", prefix)
        }
    }
}

struct LocalOAuthIdentity {
    account_id: Option<String>,
    email: Option<String>,
    plan_type: Option<String>,
}
