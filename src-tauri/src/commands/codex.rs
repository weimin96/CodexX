use chrono::Utc;
use serde_json::Value;
use tauri::State;
use uuid::Uuid;

use crate::account::{Account, AccountRepository};
use crate::codex_runtime::{
    open_codex_cli_terminal, open_codex_desktop_app, open_interactive_codex, prompt_preview,
    read_codex_launcher_config, run_codex_exec, CodexAppLaunchInput, CodexCliLaunchInput,
    CodexCommandTarget, CodexExecInput, CodexInteractiveInput, CodexLaunchResult,
};
use crate::error::AppError;
use crate::local_sync::LocalAuthSyncService;
use crate::usage::{CodexLaunchSessionRecord, UsageRepository};
use crate::AppState;

const SHORT_CONVERSATION_PROMPT: &str = "hi";
const SHORT_CONVERSATION_MODEL: &str = "gpt-5.3-codex";
const SHORT_CONVERSATION_MODEL_LABEL: &str = "GPT-5.3-Codex";
const LOW_REASONING_OVERRIDE: &str = "model_reasoning_effort=\"low\"";

#[tauri::command]
pub async fn run_codex_exec_session(
    state: State<'_, AppState>,
    input: CodexExecInput,
) -> Result<Value, AppError> {
    if input.prompt.trim().is_empty() {
        return Err(AppError::InvalidInput(
            "请输入要交给 Codex 执行的任务".to_string(),
        ));
    }

    let target = CodexCommandTarget::discover()?;
    let session_id = Uuid::new_v4().to_string();
    let started_at = Utc::now().to_rfc3339();
    let executable_label = target.executable_label();

    {
        let db = state.db.lock().await;
        let account_repo = AccountRepository::new(&db);
        account_repo.get_by_id(&input.account_id)?;
        let usage_repo = UsageRepository::new(&db);
        usage_repo.insert_launch_session(&CodexLaunchSessionRecord {
            id: session_id.clone(),
            account_id: input.account_id.clone(),
            launch_mode: "exec_json".to_string(),
            executable: Some(executable_label.clone()),
            working_directory: input.working_directory.clone(),
            prompt_preview: prompt_preview(Some(&input.prompt)),
            status: "running".to_string(),
            started_at: started_at.clone(),
            completed_at: None,
            exit_code: None,
            usage_event_count: 0,
            error_message: None,
        })?;
    }

    let outcome = run_codex_exec(&target, &input, &session_id, &started_at).await;
    let completed_at = Utc::now().to_rfc3339();

    match outcome {
        Ok(outcome) => {
            let status = if outcome.exit_code == Some(0) {
                "completed"
            } else {
                "failed"
            };
            let usage_event_count = i64::try_from(outcome.usage_events.len()).unwrap_or(0);
            {
                let db = state.db.lock().await;
                let usage_repo = UsageRepository::new(&db);
                for event in &outcome.usage_events {
                    usage_repo.insert_api_usage_event(event)?;
                }
                usage_repo.insert_launch_session(&CodexLaunchSessionRecord {
                    id: session_id.clone(),
                    account_id: input.account_id.clone(),
                    launch_mode: "exec_json".to_string(),
                    executable: Some(executable_label),
                    working_directory: input.working_directory.clone(),
                    prompt_preview: prompt_preview(Some(&input.prompt)),
                    status: status.to_string(),
                    started_at,
                    completed_at: Some(completed_at),
                    exit_code: outcome.exit_code,
                    usage_event_count,
                    error_message: outcome.stderr_preview.clone(),
                })?;
            }

            Ok(serde_json::to_value(CodexLaunchResult {
                session_id,
                status: status.to_string(),
                exit_code: outcome.exit_code,
                usage_event_count: outcome.usage_events.len(),
                message: outcome.message,
                stderr_preview: outcome.stderr_preview,
            })?)
        }
        Err(error) => {
            {
                let db = state.db.lock().await;
                let usage_repo = UsageRepository::new(&db);
                usage_repo.insert_launch_session(&CodexLaunchSessionRecord {
                    id: session_id.clone(),
                    account_id: input.account_id.clone(),
                    launch_mode: "exec_json".to_string(),
                    executable: Some(executable_label),
                    working_directory: input.working_directory.clone(),
                    prompt_preview: prompt_preview(Some(&input.prompt)),
                    status: "failed".to_string(),
                    started_at,
                    completed_at: Some(completed_at),
                    exit_code: None,
                    usage_event_count: 0,
                    error_message: Some(error.to_string()),
                })?;
            }
            Err(error)
        }
    }
}

#[tauri::command]
pub async fn trigger_codex_short_conversation(
    state: State<'_, AppState>,
    account_id: Option<String>,
) -> Result<Value, AppError> {
    let target = CodexCommandTarget::discover()?;
    let session_id = Uuid::new_v4().to_string();
    let started_at = Utc::now().to_rfc3339();
    let executable_label = target.executable_label();
    let selected_account = {
        let db = state.db.lock().await;
        let account_repo = AccountRepository::new(&db);
        let accounts = account_repo.list_all()?;
        let selected_account = select_quota_ready_account(&accounts, account_id.as_deref())?;
        LocalAuthSyncService::write_account_to_default_auth_file(
            &account_repo,
            &selected_account.id,
        )?;

        let usage_repo = UsageRepository::new(&db);
        usage_repo.insert_launch_session(&CodexLaunchSessionRecord {
            id: session_id.clone(),
            account_id: selected_account.id.clone(),
            launch_mode: "short_conversation".to_string(),
            executable: Some(executable_label.clone()),
            working_directory: None,
            prompt_preview: prompt_preview(Some(SHORT_CONVERSATION_PROMPT)),
            status: "running".to_string(),
            started_at: started_at.clone(),
            completed_at: None,
            exit_code: None,
            usage_event_count: 0,
            error_message: None,
        })?;

        selected_account
    };
    let input = CodexExecInput {
        account_id: selected_account.id.clone(),
        prompt: SHORT_CONVERSATION_PROMPT.to_string(),
        working_directory: None,
        model: Some(SHORT_CONVERSATION_MODEL.to_string()),
        profile: None,
        sandbox: Some("read-only".to_string()),
        config_overrides: Some(vec![LOW_REASONING_OVERRIDE.to_string()]),
        skip_git_repo_check: Some(true),
    };
    let outcome = run_codex_exec(&target, &input, &session_id, &started_at).await;
    let completed_at = Utc::now().to_rfc3339();

    match outcome {
        Ok(outcome) => {
            let status = if outcome.exit_code == Some(0) {
                "completed"
            } else {
                "failed"
            };
            let usage_event_count = i64::try_from(outcome.usage_events.len()).unwrap_or(0);
            {
                let db = state.db.lock().await;
                let usage_repo = UsageRepository::new(&db);
                for event in &outcome.usage_events {
                    usage_repo.insert_api_usage_event(event)?;
                }
                usage_repo.insert_launch_session(&CodexLaunchSessionRecord {
                    id: session_id.clone(),
                    account_id: selected_account.id.clone(),
                    launch_mode: "short_conversation".to_string(),
                    executable: Some(executable_label),
                    working_directory: None,
                    prompt_preview: prompt_preview(Some(SHORT_CONVERSATION_PROMPT)),
                    status: status.to_string(),
                    started_at,
                    completed_at: Some(completed_at),
                    exit_code: outcome.exit_code,
                    usage_event_count,
                    error_message: outcome.stderr_preview.clone(),
                })?;
            }

            Ok(serde_json::to_value(serde_json::json!({
                "account_id": selected_account.id,
                "account_name": selected_account.name,
                "model": SHORT_CONVERSATION_MODEL_LABEL,
                "session_id": session_id,
                "status": status,
                "exit_code": outcome.exit_code,
                "usage_event_count": outcome.usage_events.len(),
                "message": outcome.message,
                "stderr_preview": outcome.stderr_preview,
            }))?)
        }
        Err(error) => {
            {
                let db = state.db.lock().await;
                let usage_repo = UsageRepository::new(&db);
                usage_repo.insert_launch_session(&CodexLaunchSessionRecord {
                    id: session_id.clone(),
                    account_id: selected_account.id.clone(),
                    launch_mode: "short_conversation".to_string(),
                    executable: Some(executable_label),
                    working_directory: None,
                    prompt_preview: prompt_preview(Some(SHORT_CONVERSATION_PROMPT)),
                    status: "failed".to_string(),
                    started_at,
                    completed_at: Some(completed_at),
                    exit_code: None,
                    usage_event_count: 0,
                    error_message: Some(error.to_string()),
                })?;
            }
            Err(error)
        }
    }
}

fn select_quota_ready_account(
    accounts: &[Account],
    requested_account_id: Option<&str>,
) -> Result<Account, AppError> {
    if let Some(account_id) = requested_account_id {
        let account = accounts
            .iter()
            .find(|account| account.id == account_id)
            .ok_or_else(|| AppError::AccountNotFound(account_id.to_string()))?;
        if !has_full_five_hour_quota(account) {
            return Err(AppError::InvalidInput(
                "该账号 5 小时剩余额度不是 100%，不能一键预热".to_string(),
            ));
        }
        return Ok(account.clone());
    }

    accounts
        .iter()
        .find(|account| has_full_five_hour_quota(account))
        .cloned()
        .ok_or_else(|| AppError::InvalidInput("没有找到 5 小时剩余额度 100% 的账号".to_string()))
}

fn has_full_five_hour_quota(account: &Account) -> bool {
    account
        .codex_usage_5h
        .as_ref()
        .is_some_and(|usage| usage.used_percent <= 0.000_001)
}

#[tauri::command]
pub async fn open_codex_interactive_session(
    state: State<'_, AppState>,
    input: CodexInteractiveInput,
) -> Result<Value, AppError> {
    let target = CodexCommandTarget::discover()?;
    let session_id = Uuid::new_v4().to_string();
    let started_at = Utc::now().to_rfc3339();
    let executable_label = target.executable_label();

    {
        let db = state.db.lock().await;
        let account_repo = AccountRepository::new(&db);
        account_repo.get_by_id(&input.account_id)?;
    }

    open_interactive_codex(&target, &input)?;

    {
        let db = state.db.lock().await;
        let usage_repo = UsageRepository::new(&db);
        usage_repo.insert_launch_session(&CodexLaunchSessionRecord {
            id: session_id.clone(),
            account_id: input.account_id.clone(),
            launch_mode: "interactive_terminal".to_string(),
            executable: Some(executable_label),
            working_directory: input.working_directory.clone(),
            prompt_preview: prompt_preview(input.prompt.as_deref()),
            status: "launched".to_string(),
            started_at,
            completed_at: None,
            exit_code: None,
            usage_event_count: 0,
            error_message: Some("交互会话已启动，Token 用量需通过后续日志导入".to_string()),
        })?;
    }

    Ok(serde_json::to_value(CodexLaunchResult {
        session_id,
        status: "launched".to_string(),
        exit_code: None,
        usage_event_count: 0,
        message: "已在新终端中启动交互式 Codex，会话用量暂未导入".to_string(),
        stderr_preview: None,
    })?)
}

#[tauri::command]
pub async fn launch_codex_cli(
    state: State<'_, AppState>,
    input: CodexCliLaunchInput,
) -> Result<Value, AppError> {
    let target = CodexCommandTarget::discover()?;
    let session_id = Uuid::new_v4().to_string();
    let started_at = Utc::now().to_rfc3339();
    let selected_account = prepare_launch_account(&state, input.account_id.as_deref()).await?;
    if let Some(account) = selected_account.as_ref() {
        let db = state.db.lock().await;
        let usage_repo = UsageRepository::new(&db);
        usage_repo.insert_launch_session(&CodexLaunchSessionRecord {
            id: session_id.clone(),
            account_id: account.id.clone(),
            launch_mode: "cli_terminal".to_string(),
            executable: Some(target.executable_label()),
            working_directory: input.working_directory.clone(),
            prompt_preview: None,
            status: "launched".to_string(),
            started_at: started_at.clone(),
            completed_at: None,
            exit_code: None,
            usage_event_count: 0,
            error_message: Some("等待从 Codex 会话日志导入 Token 用量".to_string()),
        })?;
    }

    open_codex_cli_terminal(&target, &input)?;

    Ok(serde_json::to_value(CodexLaunchResult {
        session_id,
        status: "launched".to_string(),
        exit_code: None,
        usage_event_count: 0,
        message: "已在新终端中启动 Codex CLI".to_string(),
        stderr_preview: None,
    })?)
}

#[tauri::command]
pub async fn get_codex_launcher_config() -> Result<Value, AppError> {
    Ok(serde_json::to_value(read_codex_launcher_config()?)?)
}

#[tauri::command]
pub async fn launch_codex_app(
    state: State<'_, AppState>,
    input: CodexAppLaunchInput,
) -> Result<Value, AppError> {
    let session_id = Uuid::new_v4().to_string();
    let started_at = Utc::now().to_rfc3339();
    let selected_account = prepare_launch_account(&state, input.account_id.as_deref()).await?;
    if let Some(account) = selected_account.as_ref() {
        let db = state.db.lock().await;
        let usage_repo = UsageRepository::new(&db);
        usage_repo.insert_launch_session(&CodexLaunchSessionRecord {
            id: session_id.clone(),
            account_id: account.id.clone(),
            launch_mode: "codex_app".to_string(),
            executable: Some("Codex App".to_string()),
            working_directory: None,
            prompt_preview: None,
            status: "launched".to_string(),
            started_at,
            completed_at: None,
            exit_code: None,
            usage_event_count: 0,
            error_message: Some("等待从 Codex 会话日志导入 Token 用量".to_string()),
        })?;
    }

    open_codex_desktop_app()?;

    Ok(serde_json::to_value(CodexLaunchResult {
        session_id,
        status: "launched".to_string(),
        exit_code: None,
        usage_event_count: 0,
        message: "已启动 Codex App".to_string(),
        stderr_preview: None,
    })?)
}

async fn prepare_launch_account(
    state: &State<'_, AppState>,
    account_id: Option<&str>,
) -> Result<Option<Account>, AppError> {
    let db = state.db.lock().await;
    let account_repo = AccountRepository::new(&db);
    let accounts = account_repo.list_all()?;
    let selected_account = if let Some(account_id) = account_id {
        Some(
            accounts
                .iter()
                .find(|account| account.id == account_id)
                .cloned()
                .ok_or_else(|| AppError::AccountNotFound(account_id.to_string()))?,
        )
    } else {
        accounts
            .iter()
            .find(|account| account.is_default)
            .or_else(|| accounts.first())
            .cloned()
    };

    if let Some(account) = selected_account.as_ref() {
        LocalAuthSyncService::write_account_to_default_auth_file(&account_repo, &account.id)?;
    }

    Ok(selected_account)
}
