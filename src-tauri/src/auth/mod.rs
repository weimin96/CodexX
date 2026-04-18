use base64::engine::general_purpose::{URL_SAFE, URL_SAFE_NO_PAD};
use base64::Engine;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

use crate::account::{Account, AuthType};
use crate::error::{AppError, AppResult};

const OAUTH_ISSUER: &str = "https://auth.openai.com";
const OAUTH_CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
const OAUTH_SCOPE: &str = "openid profile email offline_access";
const OAUTH_ORIGINATOR: &str = "codex_vscode";
const OAUTH_REDIRECT_PORT: u16 = 1455;
const OAUTH_TIMEOUT_SECS: i64 = 300;
const OAUTH_CALLBACK_PATH: &str = "/auth/callback";

#[derive(Debug, Clone)]
pub struct PendingOAuthLogin {
    pub redirect_uri: String,
    pub state: String,
    pub code_verifier: String,
    pub expires_at: i64,
}

#[derive(Debug, Serialize)]
pub struct PreparedOAuthLogin {
    pub auth_url: String,
    pub redirect_uri: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct OAuthLoginResult {
    pub account_id: String,
    pub account_name: String,
    pub auth_type: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct OAuthCallbackFinishedEvent {
    pub result: Option<OAuthLoginResult>,
    pub error: Option<String>,
}

#[derive(Debug)]
pub struct CompletedOAuthLogin {
    pub auth_json: Value,
    pub account_id: String,
    pub email: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthStatus {
    Valid,
    Expired,
    Invalid,
    Unknown,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthCheckResult {
    pub account_id: String,
    pub status: AuthStatus,
    pub message: Option<String>,
    pub expires_at: Option<String>,
}

pub struct AuthService {
    client: Client,
}

pub fn oauth_redirect_port() -> u16 {
    OAUTH_REDIRECT_PORT
}

fn oauth_redirect_uri(port: u16) -> String {
    format!("http://localhost:{port}{OAUTH_CALLBACK_PATH}")
}

pub fn prepare_oauth_login(
    redirect_port: u16,
) -> AppResult<(PendingOAuthLogin, PreparedOAuthLogin)> {
    let state = Uuid::new_v4().simple().to_string();
    let code_verifier = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    let code_challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(code_verifier.as_bytes()));
    let redirect_uri = oauth_redirect_uri(redirect_port);
    let expires_at = current_unix_seconds()? + OAUTH_TIMEOUT_SECS;

    let mut auth_url = reqwest::Url::parse(&format!("{OAUTH_ISSUER}/oauth/authorize"))
        .map_err(|error| AppError::InvalidInput(format!("生成授权链接失败: {error}")))?;
    auth_url
        .query_pairs_mut()
        .append_pair("response_type", "code")
        .append_pair("client_id", OAUTH_CLIENT_ID)
        .append_pair("redirect_uri", &redirect_uri)
        .append_pair("scope", OAUTH_SCOPE)
        .append_pair("state", &state)
        .append_pair("code_challenge", &code_challenge)
        .append_pair("code_challenge_method", "S256")
        .append_pair("id_token_add_organizations", "true")
        .append_pair("codex_cli_simplified_flow", "true")
        .append_pair("originator", OAUTH_ORIGINATOR);

    Ok((
        PendingOAuthLogin {
            redirect_uri: redirect_uri.clone(),
            state,
            code_verifier,
            expires_at,
        },
        PreparedOAuthLogin {
            auth_url: auth_url.to_string(),
            redirect_uri,
        },
    ))
}

pub async fn complete_oauth_callback_login(
    pending: &PendingOAuthLogin,
    callback_url: &str,
) -> AppResult<CompletedOAuthLogin> {
    let callback_url = callback_url.trim();
    if callback_url.is_empty() {
        return Err(AppError::InvalidInput("请粘贴回调链接".to_string()));
    }

    let parsed_url = parse_oauth_callback_url(callback_url)?;
    let params: std::collections::HashMap<String, String> = parsed_url
        .query_pairs()
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect();

    if let Some(error) = params.get("error") {
        let description = params
            .get("error_description")
            .map(String::as_str)
            .unwrap_or(error.as_str());
        return Err(AppError::AuthFailed(format!("授权失败: {description}")));
    }

    let state = params
        .get("state")
        .ok_or_else(|| AppError::InvalidInput("回调链接缺少 state 参数".to_string()))?;
    if state != &pending.state {
        return Err(AppError::AuthFailed(
            "回调链接 state 不匹配，请重新生成授权链接".to_string(),
        ));
    }

    let code = params
        .get("code")
        .ok_or_else(|| AppError::InvalidInput("回调链接缺少 code 参数".to_string()))?;

    exchange_authorization_code(code, pending).await
}

impl AuthService {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .expect("Failed to build HTTP client"),
        }
    }

    pub async fn validate_credential(
        &self,
        account: &Account,
        credential: &str,
    ) -> AppResult<AuthCheckResult> {
        match account.auth_type {
            AuthType::ApiKey => self.validate_api_key(account, credential).await,
            AuthType::OAuthToken => self.validate_oauth_token(account, credential).await,
            AuthType::CookieSession => self.validate_session(account, credential).await,
            AuthType::CliProfile => self.validate_cli_profile(account, credential).await,
        }
    }

    async fn validate_api_key(
        &self,
        account: &Account,
        api_key: &str,
    ) -> AppResult<AuthCheckResult> {
        // 通过模型列表接口做轻量校验，避免把 API Key 明文写入日志或错误信息。
        let response = self
            .client
            .get("https://api.openai.com/v1/models")
            .header("Authorization", format!("Bearer {}", api_key))
            .send()
            .await;

        match response {
            Ok(resp) => {
                let status = resp.status();
                if status.is_success() {
                    Ok(AuthCheckResult {
                        account_id: account.id.clone(),
                        status: AuthStatus::Valid,
                        message: Some("API Key 有效".to_string()),
                        expires_at: None,
                    })
                } else if status.as_u16() == 401 {
                    Ok(AuthCheckResult {
                        account_id: account.id.clone(),
                        status: AuthStatus::Invalid,
                        message: Some("API Key 无效或已过期".to_string()),
                        expires_at: None,
                    })
                } else if status.as_u16() == 429 {
                    Ok(AuthCheckResult {
                        account_id: account.id.clone(),
                        status: AuthStatus::Valid,
                        message: Some("API Key 有效（触发限流）".to_string()),
                        expires_at: None,
                    })
                } else {
                    Ok(AuthCheckResult {
                        account_id: account.id.clone(),
                        status: AuthStatus::Unknown,
                        message: Some(format!("HTTP {}", status.as_u16())),
                        expires_at: None,
                    })
                }
            }
            Err(e) => Ok(AuthCheckResult {
                account_id: account.id.clone(),
                status: AuthStatus::Unknown,
                message: Some(format!("网络错误: {}", e)),
                expires_at: None,
            }),
        }
    }

    async fn validate_oauth_token(
        &self,
        account: &Account,
        credential: &str,
    ) -> AppResult<AuthCheckResult> {
        let token = oauth_access_token_from_credential(credential);
        let response = self
            .client
            .get("https://api.openai.com/v1/me")
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await;

        match response {
            Ok(resp) => {
                if resp.status().is_success() {
                    Ok(AuthCheckResult {
                        account_id: account.id.clone(),
                        status: AuthStatus::Valid,
                        message: Some("OAuth Token 有效".to_string()),
                        expires_at: None,
                    })
                } else {
                    Ok(AuthCheckResult {
                        account_id: account.id.clone(),
                        status: AuthStatus::Expired,
                        message: Some("Token 已过期，请重新登录".to_string()),
                        expires_at: None,
                    })
                }
            }
            Err(e) => Ok(AuthCheckResult {
                account_id: account.id.clone(),
                status: AuthStatus::Unknown,
                message: Some(format!("无法连接: {}", e)),
                expires_at: None,
            }),
        }
    }

    async fn validate_session(
        &self,
        account: &Account,
        _session: &str,
    ) -> AppResult<AuthCheckResult> {
        Ok(AuthCheckResult {
            account_id: account.id.clone(),
            status: AuthStatus::Unknown,
            message: Some("Session 验证需要手动确认".to_string()),
            expires_at: None,
        })
    }

    async fn validate_cli_profile(
        &self,
        account: &Account,
        profile: &str,
    ) -> AppResult<AuthCheckResult> {
        // CLI Profile 只能验证本地配置存在性，真实可用性仍需用户在 Codex CLI 中确认。
        let home = std::env::var("HOME").unwrap_or_default();
        let config_path = format!("{}/.codex/profiles/{}", home, profile);

        if std::path::Path::new(&config_path).exists() {
            Ok(AuthCheckResult {
                account_id: account.id.clone(),
                status: AuthStatus::Valid,
                message: Some("CLI Profile 存在".to_string()),
                expires_at: None,
            })
        } else {
            Ok(AuthCheckResult {
                account_id: account.id.clone(),
                status: AuthStatus::Invalid,
                message: Some("CLI Profile 不存在".to_string()),
                expires_at: None,
            })
        }
    }
}

impl Default for AuthService {
    fn default() -> Self {
        Self::new()
    }
}

fn parse_oauth_callback_url(callback_url: &str) -> AppResult<reqwest::Url> {
    reqwest::Url::parse(callback_url)
        .or_else(|_| reqwest::Url::parse(&format!("http://localhost{callback_url}")))
        .map_err(|error| AppError::InvalidInput(format!("回调链接格式无效: {error}")))
}

async fn exchange_authorization_code(
    code: &str,
    pending: &PendingOAuthLogin,
) -> AppResult<CompletedOAuthLogin> {
    let client = Client::builder()
        .user_agent("codex-manager/0.1.0")
        .timeout(std::time::Duration::from_secs(20))
        .build()?;
    let token_url = format!("{OAUTH_ISSUER}/oauth/token");
    let response = client
        .post(&token_url)
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", pending.redirect_uri.as_str()),
            ("client_id", OAUTH_CLIENT_ID),
            ("code_verifier", pending.code_verifier.as_str()),
        ])
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(AppError::AuthFailed(format!(
            "换取登录令牌失败 {}: {}",
            status.as_u16(),
            truncate_error_body(&body, 200)
        )));
    }

    let token_response: OAuthTokenResponse = response.json().await?;
    build_auth_json_from_oauth_tokens(token_response)
}

fn build_auth_json_from_oauth_tokens(
    token_response: OAuthTokenResponse,
) -> AppResult<CompletedOAuthLogin> {
    let id_token_claims = decode_jwt_payload(&token_response.id_token)?;
    let account_id = id_token_claims
        .get("https://api.openai.com/auth")
        .and_then(Value::as_object)
        .and_then(|auth| auth.get("chatgpt_account_id"))
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::InvalidInput("无法从 OAuth 登录结果识别账号 ID".to_string()))?
        .to_string();
    let email = id_token_claims
        .get("email")
        .and_then(Value::as_str)
        .map(ToString::to_string);

    let auth_json = serde_json::json!({
        "OPENAI_API_KEY": Value::Null,
        "auth_mode": "chatgpt",
        "last_refresh": chrono::Utc::now().to_rfc3339(),
        "tokens": {
            "access_token": token_response.access_token,
            "refresh_token": token_response.refresh_token,
            "id_token": token_response.id_token,
            "account_id": account_id
        }
    });

    Ok(CompletedOAuthLogin {
        auth_json,
        account_id,
        email,
    })
}

pub(crate) fn decode_jwt_payload(token: &str) -> AppResult<Value> {
    let payload = token
        .split('.')
        .nth(1)
        .ok_or_else(|| AppError::InvalidInput("id_token 格式无效".to_string()))?;
    let decoded = URL_SAFE_NO_PAD
        .decode(payload)
        .or_else(|_| {
            let remainder = payload.len() % 4;
            let padded = if remainder == 0 {
                payload.to_string()
            } else {
                format!("{payload}{}", "=".repeat(4 - remainder))
            };
            URL_SAFE.decode(padded)
        })
        .map_err(|error| AppError::InvalidInput(format!("解码 id_token 失败: {error}")))?;

    Ok(serde_json::from_slice(&decoded)?)
}

fn oauth_access_token_from_credential(credential: &str) -> String {
    serde_json::from_str::<Value>(credential)
        .ok()
        .and_then(|value| {
            value
                .get("tokens")
                .and_then(Value::as_object)
                .and_then(|tokens| tokens.get("access_token"))
                .or_else(|| value.get("access_token"))
                .and_then(Value::as_str)
                .map(ToString::to_string)
        })
        .unwrap_or_else(|| credential.to_string())
}

fn current_unix_seconds() -> AppResult<i64> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| AppError::Other(format!("读取系统时间失败: {error}")))?
        .as_secs() as i64)
}

fn truncate_error_body(value: &str, max_chars: usize) -> String {
    let trimmed = value.trim();
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_string();
    }

    let prefix: String = trimmed.chars().take(max_chars).collect();
    format!("{prefix}...")
}

#[derive(Debug, Deserialize)]
struct OAuthTokenResponse {
    access_token: String,
    refresh_token: String,
    id_token: String,
}
