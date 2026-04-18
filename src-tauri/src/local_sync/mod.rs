use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::account::{AccountRepository, AuthType, UpsertSyncedAccountInput};
use crate::error::{AppError, AppResult};

#[derive(Debug, Serialize)]
pub struct LocalAuthSyncResult {
    pub account_id: String,
    pub account_name: String,
    pub auth_type: String,
    pub auth_file_path: String,
}

#[derive(Debug, Deserialize)]
struct LocalAuthFileDocument {
    #[serde(rename = "OPENAI_API_KEY")]
    openai_api_key: Option<String>,
    auth_mode: Option<String>,
    tokens: Option<LocalAuthTokens>,
}

#[derive(Debug, Deserialize)]
struct LocalAuthTokens {
    access_token: Option<String>,
    account_id: Option<String>,
}

struct LocalAuthAccountDraft {
    stable_id: String,
    name: String,
    auth_type: AuthType,
    organization: Option<String>,
    credential_value: String,
}

pub struct LocalAuthSyncService;

impl LocalAuthSyncService {
    pub fn sync_auth_file(
        repo: &AccountRepository<'_>,
        auth_file_path: Option<&str>,
    ) -> AppResult<LocalAuthSyncResult> {
        let resolved_path = Self::resolve_auth_file_path(auth_file_path)?;
        let auth_document = Self::read_auth_document(&resolved_path)?;
        let account_draft = Self::build_account_draft(&resolved_path, auth_document)?;
        let account = repo.upsert_synced_account(UpsertSyncedAccountInput {
            stable_id: account_draft.stable_id,
            name: account_draft.name,
            auth_type: account_draft.auth_type,
            email: None,
            organization: account_draft.organization,
            color: Some("#4f8ef7".to_string()),
            credential_value: account_draft.credential_value,
            credential_type: None,
        })?;

        Ok(LocalAuthSyncResult {
            account_id: account.id,
            account_name: account.name,
            auth_type: account.auth_type.to_string(),
            auth_file_path: resolved_path.to_string_lossy().to_string(),
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
            "无法推断本地 auth.json 路径，请在设置中手动填写".to_string(),
        ))
    }

    fn read_auth_document(path: &Path) -> AppResult<LocalAuthFileDocument> {
        if !path.exists() {
            return Err(AppError::InvalidInput(format!(
                "未找到 auth.json 文件：{}",
                path.to_string_lossy()
            )));
        }

        let raw = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(&raw)?)
    }

    fn build_account_draft(
        path: &Path,
        auth_document: LocalAuthFileDocument,
    ) -> AppResult<LocalAuthAccountDraft> {
        let stable_id = Self::build_stable_account_id(path);
        let source_path = Self::normalize_path(path);

        if let Some(api_key) = Self::non_empty_text(auth_document.openai_api_key.as_deref()) {
            return Ok(LocalAuthAccountDraft {
                stable_id,
                name: "本地 auth.json（API Key）".to_string(),
                auth_type: AuthType::ApiKey,
                organization: Some("本地文件同步".to_string()),
                credential_value: api_key.to_string(),
            });
        }

        if let Some(token) = auth_document
            .tokens
            .as_ref()
            .and_then(|tokens| Self::non_empty_text(tokens.access_token.as_deref()))
        {
            let account_hint = auth_document
                .tokens
                .as_ref()
                .and_then(|tokens| tokens.account_id.as_deref())
                .and_then(|account_id| Self::non_empty_text(Some(account_id)))
                .map(Self::shorten_account_hint);

            let organization = match account_hint {
                Some(account_id) => Some(format!("本地文件同步 · {}", account_id)),
                None => Some("本地文件同步".to_string()),
            };

            return Ok(LocalAuthAccountDraft {
                stable_id,
                name: "本地 auth.json（OAuth）".to_string(),
                auth_type: AuthType::OAuthToken,
                organization,
                credential_value: token.to_string(),
            });
        }

        Err(AppError::InvalidInput(format!(
            "auth.json 中未找到可同步的凭证字段：{}，当前模式 {:?}",
            source_path.to_string_lossy(),
            auth_document.auth_mode
        )))
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
