use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::account::{Account, AccountRepository, AccountStatus, AuthType};
use crate::error::AppResult;
use crate::storage::Database;

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
        // Try to hit the OpenAI/Codex models endpoint
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
        token: &str,
    ) -> AppResult<AuthCheckResult> {
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
        // Check if the CLI profile file exists
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
