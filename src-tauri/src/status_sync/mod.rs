use chrono::Utc;
use serde_json::Value;

use crate::account::{
    Account, AccountRepository, AccountStatus, CodexAccountProfile, UpdateAccountInput,
};
use crate::auth::{self, AuthService, AuthStatus};
use crate::codex_usage;
use crate::error::{AppError, AppResult};
use crate::local_sync::LocalAuthSyncService;
use crate::AppState;

pub struct AccountRefreshOutcome {
    pub status: AccountStatus,
    pub message: Option<String>,
    pub codex_profile: Option<CodexAccountProfile>,
}

pub async fn load_account_and_credential(
    state: &AppState,
    account_id: &str,
) -> Result<(Account, String), AppError> {
    let account = {
        let db = state.db.lock().await;
        let repo = AccountRepository::new(&db);
        let account = repo.get_by_id(account_id)?;
        match repo.get_credential(account_id) {
            Ok(credential) => return Ok((account, credential)),
            Err(AppError::Security(_)) => account,
            Err(error) => return Err(error),
        }
    };

    recover_local_auth_credential(state, &account).await
}

pub async fn evaluate_account_refresh(
    account: &Account,
    credential: &str,
) -> Result<AccountRefreshOutcome, AppError> {
    let auth_service = AuthService::new();
    let auth_result = auth_service
        .validate_credential(account, credential)
        .await?;
    let codex_profile = refresh_codex_profile(account, credential).await;
    let status = resolve_checked_status(&auth_result.status, codex_profile.as_ref());
    let message = if status == AccountStatus::Normal {
        None
    } else {
        compose_status_message(auth_result.message.as_deref(), codex_profile.as_ref())
    };

    Ok(AccountRefreshOutcome {
        status,
        message,
        codex_profile,
    })
}

pub fn persist_refresh_outcome(
    repo: &AccountRepository<'_>,
    account_id: &str,
    outcome: &AccountRefreshOutcome,
) -> AppResult<Account> {
    repo.update_status(account_id, &outcome.status, outcome.message.as_deref())?;
    if let Some(profile) = outcome.codex_profile.as_ref() {
        repo.update_codex_profile(account_id, profile)?;
    }

    // 持久化后重新读取账号快照，确保事件载荷与数据库中的最终状态完全一致。
    repo.get_by_id(account_id)
}

async fn recover_local_auth_credential(
    state: &AppState,
    account: &Account,
) -> Result<(Account, String), AppError> {
    if !account.id.starts_with("local-auth-") {
        return Err(AppError::Security(
            "本地加密凭证无法解密，请重新登录或重新导入账号".to_string(),
        ));
    }

    let prepared = LocalAuthSyncService::prepare_auth_file(None).map_err(|error| {
        AppError::Security(format!(
            "本地加密凭证无法解密，并且无法读取默认 auth.json 自动恢复: {error}"
        ))
    })?;

    if !prepared.can_recover_account(account) {
        return Err(AppError::Security(
            "本地加密凭证无法解密，当前默认 auth.json 与该账号身份不匹配，请点击本地同步新增或更新当前账号".to_string(),
        ));
    }

    let credential = prepared.credential_value().to_string();
    let recovered_account = {
        let db = state.db.lock().await;
        let repo = AccountRepository::new(&db);
        if prepared.stable_id() == account.id {
            let result = LocalAuthSyncService::sync_prepared_auth(&repo, prepared, None)?;
            repo.get_by_id(&result.account_id)?
        } else {
            repo.update(UpdateAccountInput {
                id: account.id.clone(),
                name: None,
                email: None,
                organization: None,
                color: None,
                credential_value: Some(credential.clone()),
            })?
        }
    };

    Ok((recovered_account, credential))
}

async fn refresh_codex_profile(account: &Account, credential: &str) -> Option<CodexAccountProfile> {
    let source = extract_codex_usage_source(credential, account.codex_plan_type.clone())?;

    match codex_usage::fetch_codex_account_profile(
        &source.access_token,
        &source.chatgpt_account_id,
        source.fallback_plan_type.clone(),
    )
    .await
    {
        Ok(profile) => Some(profile),
        Err(error) => Some(CodexAccountProfile {
            plan_type: source.fallback_plan_type,
            fetched_at: Some(Utc::now().to_rfc3339()),
            five_hour: None,
            one_week: None,
            usage_error: Some(error),
        }),
    }
}

fn extract_codex_usage_source(
    credential: &str,
    current_plan_type: Option<String>,
) -> Option<CodexUsageRefreshSource> {
    let credential_json = serde_json::from_str::<Value>(credential).ok()?;
    let tokens = credential_json.get("tokens").and_then(Value::as_object);

    let access_token = tokens
        .and_then(|tokens| tokens.get("access_token"))
        .or_else(|| credential_json.get("access_token"))
        .and_then(Value::as_str)
        .and_then(non_empty_text)
        .map(ToString::to_string)?;

    let id_token = tokens
        .and_then(|tokens| tokens.get("id_token"))
        .or_else(|| credential_json.get("id_token"))
        .and_then(Value::as_str)
        .and_then(non_empty_text);
    let claims = id_token.and_then(|token| auth::decode_jwt_payload(token).ok());
    let auth_claim = claims
        .as_ref()
        .and_then(|value| value.get("https://api.openai.com/auth"))
        .and_then(Value::as_object);

    let chatgpt_account_id = tokens
        .and_then(|tokens| tokens.get("account_id"))
        .or_else(|| credential_json.get("account_id"))
        .and_then(Value::as_str)
        .and_then(non_empty_text)
        .map(ToString::to_string)
        .or_else(|| {
            auth_claim
                .and_then(|value| value.get("chatgpt_account_id"))
                .and_then(Value::as_str)
                .and_then(non_empty_text)
                .map(ToString::to_string)
        })?;

    let fallback_plan_type = current_plan_type.or_else(|| {
        auth_claim
            .and_then(|value| value.get("chatgpt_plan_type"))
            .and_then(Value::as_str)
            .and_then(non_empty_text)
            .map(ToString::to_string)
    });

    Some(CodexUsageRefreshSource {
        access_token,
        chatgpt_account_id,
        fallback_plan_type,
    })
}

fn resolve_checked_status(
    auth_status: &AuthStatus,
    codex_profile: Option<&CodexAccountProfile>,
) -> AccountStatus {
    if let Some(profile) = codex_profile {
        if profile.usage_error.is_none() {
            return AccountStatus::Normal;
        }

        if profile
            .usage_error
            .as_deref()
            .is_some_and(|error| error.contains("登录信息已失效"))
        {
            return AccountStatus::Expired;
        }
    }

    match auth_status {
        AuthStatus::Valid => AccountStatus::Normal,
        AuthStatus::Expired => AccountStatus::Expired,
        AuthStatus::Invalid => AccountStatus::Error,
        AuthStatus::Unknown => AccountStatus::Unknown,
    }
}

fn compose_status_message(
    auth_message: Option<&str>,
    codex_profile: Option<&CodexAccountProfile>,
) -> Option<String> {
    match codex_profile {
        Some(profile) if profile.usage_error.is_none() => None,
        Some(profile) => {
            let usage_message = profile
                .usage_error
                .as_deref()
                .unwrap_or("Codex 资料接口暂不可用，可稍后重试");
            Some(match auth_message {
                Some(base) => format!("{base}，{usage_message}"),
                None => usage_message.to_string(),
            })
        }
        None => auth_message.map(ToString::to_string),
    }
}

fn non_empty_text(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

struct CodexUsageRefreshSource {
    access_token: String,
    chatgpt_account_id: String,
    fallback_plan_type: Option<String>,
}
