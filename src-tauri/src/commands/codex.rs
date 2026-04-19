use chrono::Utc;
use serde_json::Value;
use tauri::State;
use uuid::Uuid;

use crate::account::AccountRepository;
use crate::codex_runtime::{
    open_codex_cli_terminal, open_codex_desktop_app, open_interactive_codex, prompt_preview,
    run_codex_exec, CodexCliLaunchInput, CodexCommandTarget, CodexExecInput,
    CodexInteractiveInput, CodexLaunchResult,
};
use crate::error::AppError;
use crate::usage::{CodexLaunchSessionRecord, UsageRepository};
use crate::AppState;

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
pub async fn launch_codex_cli(input: CodexCliLaunchInput) -> Result<Value, AppError> {
    let target = CodexCommandTarget::discover()?;
    open_codex_cli_terminal(&target, &input)?;

    Ok(serde_json::to_value(CodexLaunchResult {
        session_id: Uuid::new_v4().to_string(),
        status: "launched".to_string(),
        exit_code: None,
        usage_event_count: 0,
        message: "已在新终端中启动 Codex CLI".to_string(),
        stderr_preview: None,
    })?)
}

#[tauri::command]
pub async fn launch_codex_app() -> Result<Value, AppError> {
    open_codex_desktop_app()?;

    Ok(serde_json::to_value(CodexLaunchResult {
        session_id: Uuid::new_v4().to_string(),
        status: "launched".to_string(),
        exit_code: None,
        usage_event_count: 0,
        message: "已启动 Codex App".to_string(),
        stderr_preview: None,
    })?)
}
