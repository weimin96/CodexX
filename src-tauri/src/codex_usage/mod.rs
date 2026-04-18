use chrono::Utc;
use serde::Deserialize;

use crate::account::{CodexAccountProfile, CodexUsageWindow};

const CODEX_USAGE_URLS: [&str; 2] = [
    "https://chatgpt.com/backend-api/wham/usage",
    "https://chatgpt.com/api/codex/usage",
];

#[derive(Debug, Deserialize)]
struct CodexUsageApiResponse {
    plan_type: Option<String>,
    rate_limit: Option<RateLimitDetails>,
    additional_rate_limits: Option<Vec<AdditionalRateLimitDetails>>,
}

#[derive(Debug, Deserialize)]
struct RateLimitDetails {
    primary_window: Option<UsageWindowRaw>,
    secondary_window: Option<UsageWindowRaw>,
}

#[derive(Debug, Deserialize)]
struct AdditionalRateLimitDetails {
    rate_limit: Option<RateLimitDetails>,
}

#[derive(Debug, Clone, Deserialize)]
struct UsageWindowRaw {
    used_percent: f64,
    limit_window_seconds: i64,
    reset_at: i64,
}

pub async fn fetch_codex_account_profile(
    access_token: &str,
    account_id: &str,
    fallback_plan_type: Option<String>,
) -> Result<CodexAccountProfile, String> {
    let client = reqwest::Client::builder()
        .user_agent("codex-manager/0.1.0")
        .timeout(std::time::Duration::from_secs(18))
        .build()
        .map_err(|error| format!("创建资料请求客户端失败: {error}"))?;
    let mut errors = Vec::new();

    for usage_url in CODEX_USAGE_URLS {
        let response = match client
            .get(usage_url)
            .header("Authorization", format!("Bearer {access_token}"))
            .header("ChatGPT-Account-Id", account_id)
            .header("Accept", "application/json")
            .send()
            .await
        {
            Ok(response) => response,
            Err(error) => {
                errors.push(CodexUsageFailure::transport(usage_url, &error.to_string()));
                continue;
            }
        };

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            errors.push(CodexUsageFailure::http(
                usage_url,
                status.as_u16(),
                &truncate_error_body(&body, 180),
            ));
            continue;
        }

        let payload: CodexUsageApiResponse = match response.json().await {
            Ok(payload) => payload,
            Err(error) => {
                errors.push(CodexUsageFailure::parse(usage_url, &error.to_string()));
                continue;
            }
        };
        return Ok(map_usage_payload(payload, fallback_plan_type));
    }

    Err(summarize_failures(&errors))
}

fn map_usage_payload(
    payload: CodexUsageApiResponse,
    fallback_plan_type: Option<String>,
) -> CodexAccountProfile {
    let mut windows = Vec::new();

    if let Some(rate_limit) = payload.rate_limit {
        collect_rate_limit_windows(rate_limit, &mut windows);
    }

    if let Some(additional_rate_limits) = payload.additional_rate_limits {
        for additional in additional_rate_limits {
            if let Some(rate_limit) = additional.rate_limit {
                collect_rate_limit_windows(rate_limit, &mut windows);
            }
        }
    }

    CodexAccountProfile {
        plan_type: payload.plan_type.or(fallback_plan_type),
        fetched_at: Some(Utc::now().to_rfc3339()),
        five_hour: pick_nearest_window(&windows, 5 * 60 * 60),
        one_week: pick_nearest_window(&windows, 7 * 24 * 60 * 60),
        usage_error: None,
    }
}

fn collect_rate_limit_windows(rate_limit: RateLimitDetails, windows: &mut Vec<UsageWindowRaw>) {
    if let Some(primary_window) = rate_limit.primary_window {
        windows.push(primary_window);
    }
    if let Some(secondary_window) = rate_limit.secondary_window {
        windows.push(secondary_window);
    }
}

fn pick_nearest_window(
    windows: &[UsageWindowRaw],
    target_window_seconds: i64,
) -> Option<CodexUsageWindow> {
    windows
        .iter()
        .min_by_key(|window| (window.limit_window_seconds - target_window_seconds).abs())
        .map(|window| CodexUsageWindow {
            used_percent: window.used_percent,
            window_seconds: window.limit_window_seconds,
            reset_at: Some(window.reset_at),
        })
}

fn summarize_failures(errors: &[CodexUsageFailure]) -> String {
    if errors.is_empty() {
        return "Codex 资料接口暂不可用，可稍后重试".to_string();
    }

    if errors
        .iter()
        .any(|error| matches!(error.status_code(), Some(401 | 403)))
    {
        return "登录信息已失效，请重新同步本地账号".to_string();
    }

    if errors
        .iter()
        .any(|error| matches!(error.status_code(), Some(status) if status >= 500))
    {
        return "Codex 资料接口暂时不可用，可稍后重试".to_string();
    }

    if errors
        .iter()
        .all(|error| matches!(error.kind, CodexUsageFailureKind::Transport))
    {
        return "Codex 资料接口暂不可达，可稍后重试".to_string();
    }

    if errors
        .iter()
        .any(|error| matches!(error.kind, CodexUsageFailureKind::Parse))
    {
        return "Codex 资料接口返回格式暂不兼容".to_string();
    }

    "Codex 资料接口返回异常，可稍后重试".to_string()
}

fn truncate_error_body(value: &str, max_chars: usize) -> String {
    let trimmed = value.trim();
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_string();
    }

    let prefix = trimmed.chars().take(max_chars).collect::<String>();
    format!("{prefix}...")
}

#[derive(Debug)]
struct CodexUsageFailure {
    kind: CodexUsageFailureKind,
    status_code: Option<u16>,
}

impl CodexUsageFailure {
    fn transport(_url: &str, _detail: &str) -> Self {
        Self {
            kind: CodexUsageFailureKind::Transport,
            status_code: None,
        }
    }

    fn http(_url: &str, status_code: u16, _detail: &str) -> Self {
        Self {
            kind: CodexUsageFailureKind::Http,
            status_code: Some(status_code),
        }
    }

    fn parse(_url: &str, _detail: &str) -> Self {
        Self {
            kind: CodexUsageFailureKind::Parse,
            status_code: None,
        }
    }

    fn status_code(&self) -> Option<u16> {
        self.status_code
    }
}

#[derive(Debug)]
enum CodexUsageFailureKind {
    Transport,
    Http,
    Parse,
}
