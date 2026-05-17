use chrono::Utc;
use serde::Serialize;
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration as StdDuration;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::account::{Account, AccountRepository};
use crate::codex_runtime::{
    close_codex_desktop_app, open_codex_cli_terminal, open_codex_desktop_app,
    open_interactive_codex, prompt_preview, read_codex_launcher_config,
    read_existing_trusted_project_paths, run_codex_exec, CodexAppLaunchInput, CodexCliLaunchInput,
    CodexCommandTarget, CodexExecInput, CodexInteractiveInput, CodexLaunchResult,
};
use crate::codex_session_import::import_codex_session_usage_for_session;
use crate::error::AppError;
use crate::local_sync::LocalAuthSyncService;
use crate::status_sync;
use crate::storage::Database;
use crate::usage::{CodexLaunchSessionRecord, UsageImportSession, UsageRepository};
use crate::AppState;

const SHORT_CONVERSATION_PROMPT: &str = "hi";
const SHORT_CONVERSATION_MODEL: &str = "gpt-5.2";
const SHORT_CONVERSATION_MODEL_LABEL: &str = "GPT-5.2";
const LOW_REASONING_OVERRIDE: &str = "model_reasoning_effort=\"low\"";
const PERIOD_WARMUP_LABEL: &str = "周期";
const PERIOD_WARMUP_REMAINING_QUOTA_EPSILON: f64 = 0.000_001;
const SESSION_USAGE_IMPORT_ATTEMPTS: usize = 12;
const SESSION_USAGE_IMPORT_INTERVAL_SECONDS: u64 = 15;

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
enum WarmupWorkingDirectorySource {
    RecentSession,
    TrustedProject,
    ProcessCwd,
}

#[derive(Debug, Clone)]
struct WarmupWorkspaceSelection {
    working_directory: Option<String>,
    source: WarmupWorkingDirectorySource,
}

#[derive(Debug, Clone, Copy)]
enum WarmupScope {
    Period,
}

impl WarmupScope {
    fn parse(value: Option<&str>) -> Result<Self, AppError> {
        match value.map(str::trim).filter(|value| !value.is_empty()) {
            None | Some("period") => Ok(Self::Period),
            // 历史版本曾按单个额度窗口传参；统一按周期预热处理，避免旧前端调用直接失效。
            Some("five_hour") | Some("one_week") => Ok(Self::Period),
            Some(other) => Err(AppError::InvalidInput(format!(
                "预热范围参数不合法: {other}"
            ))),
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Period => PERIOD_WARMUP_LABEL,
        }
    }
}

#[tauri::command]
pub async fn run_codex_exec_session(
    app: AppHandle,
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

            if status == "completed" {
                refresh_quota_after_codex_task(&app, state.inner(), &input.account_id).await;
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
    app: AppHandle,
    state: State<'_, AppState>,
    account_id: Option<String>,
    warmup_window: Option<String>,
) -> Result<Value, AppError> {
    let warmup_scope = WarmupScope::parse(warmup_window.as_deref())?;
    let target = CodexCommandTarget::discover()?;
    let session_id = Uuid::new_v4().to_string();
    let started_at = Utc::now().to_rfc3339();
    let now_timestamp = Utc::now().timestamp();
    let executable_label = target.executable_label();
    let (selected_account, warmup_workspace, previous_default_account_id) = {
        let db = state.db.lock().await;
        let account_repo = AccountRepository::new(&db);
        let accounts = account_repo.list_all()?;
        let previous_default_account_id = accounts
            .iter()
            .find(|account| account.is_default)
            .map(|account| account.id.clone());
        let selected_account = select_warmup_ready_account(
            &accounts,
            account_id.as_deref(),
            warmup_scope,
            now_timestamp,
        )?;
        LocalAuthSyncService::write_account_to_default_auth_file(
            &account_repo,
            &selected_account.id,
        )?;

        let usage_repo = UsageRepository::new(&db);
        let warmup_workspace = resolve_warmup_workspace(&usage_repo, selected_account.id.as_str())?;
        usage_repo.insert_launch_session(&CodexLaunchSessionRecord {
            id: session_id.clone(),
            account_id: selected_account.id.clone(),
            launch_mode: "short_conversation".to_string(),
            executable: Some(executable_label.clone()),
            working_directory: warmup_workspace.working_directory.clone(),
            prompt_preview: prompt_preview(Some(SHORT_CONVERSATION_PROMPT)),
            status: "running".to_string(),
            started_at: started_at.clone(),
            completed_at: None,
            exit_code: None,
            usage_event_count: 0,
            error_message: None,
        })?;

        (
            selected_account,
            warmup_workspace,
            previous_default_account_id,
        )
    };
    let input = CodexExecInput {
        account_id: selected_account.id.clone(),
        prompt: SHORT_CONVERSATION_PROMPT.to_string(),
        working_directory: warmup_workspace.working_directory.clone(),
        model: Some(SHORT_CONVERSATION_MODEL.to_string()),
        profile: None,
        sandbox: Some("read-only".to_string()),
        // 当前 Codex CLI 没有单独的“标准速度”快捷参数；这里不附加任何速度档位覆盖，
        // 仅显式锁定模型和低思考，让预热继续走默认速度。
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
                    working_directory: warmup_workspace.working_directory.clone(),
                    prompt_preview: prompt_preview(Some(SHORT_CONVERSATION_PROMPT)),
                    status: status.to_string(),
                    started_at,
                    completed_at: Some(completed_at),
                    exit_code: outcome.exit_code,
                    usage_event_count,
                    error_message: outcome.stderr_preview.clone(),
                })?;
            }

            if status == "completed" {
                refresh_quota_after_codex_task(&app, state.inner(), &selected_account.id).await;
            }
            restore_warmup_default_account(
                state.inner(),
                previous_default_account_id.as_deref(),
                &selected_account.id,
            )
            .await?;

            let selected_account_name = account_email_or_name(&selected_account);
            Ok(serde_json::to_value(serde_json::json!({
                "account_id": selected_account.id,
                "account_name": selected_account_name,
                "model": SHORT_CONVERSATION_MODEL_LABEL,
                "session_id": session_id,
                "status": status,
                "exit_code": outcome.exit_code,
                "usage_event_count": outcome.usage_events.len(),
                "working_directory": warmup_workspace.working_directory,
                "working_directory_source": warmup_workspace.source,
                "warmup_window": warmup_scope.label(),
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
                    working_directory: warmup_workspace.working_directory.clone(),
                    prompt_preview: prompt_preview(Some(SHORT_CONVERSATION_PROMPT)),
                    status: "failed".to_string(),
                    started_at,
                    completed_at: Some(completed_at),
                    exit_code: None,
                    usage_event_count: 0,
                    error_message: Some(error.to_string()),
                })?;
            }
            restore_warmup_default_account(
                state.inner(),
                previous_default_account_id.as_deref(),
                &selected_account.id,
            )
            .await?;
            Err(error)
        }
    }
}

async fn restore_warmup_default_account(
    state: &AppState,
    previous_default_account_id: Option<&str>,
    warmup_account_id: &str,
) -> Result<(), AppError> {
    let Some(previous_default_account_id) = previous_default_account_id else {
        return Ok(());
    };

    if previous_default_account_id == warmup_account_id {
        return Ok(());
    }

    let db = state.db.lock().await;
    let account_repo = AccountRepository::new(&db);
    LocalAuthSyncService::write_account_to_default_auth_file(
        &account_repo,
        previous_default_account_id,
    )
    .map(|_| ())
    .map_err(|error| AppError::Other(format!("周期预热已结束，但恢复原默认账号失败: {error}")))
}

fn select_warmup_ready_account(
    accounts: &[Account],
    requested_account_id: Option<&str>,
    warmup_scope: WarmupScope,
    now_timestamp: i64,
) -> Result<Account, AppError> {
    if let Some(account_id) = requested_account_id {
        let account = accounts
            .iter()
            .find(|account| account.id == account_id)
            .ok_or_else(|| AppError::AccountNotFound(account_id.to_string()))?;
        validate_warmup_scope_executable(account, warmup_scope, now_timestamp)?;
        return Ok(account.clone());
    }

    accounts
        .iter()
        .find(|account| is_warmup_scope_executable(account, warmup_scope, now_timestamp))
        .cloned()
        .ok_or_else(|| {
            AppError::InvalidInput(format!(
                "没有找到两个周期都有剩余额度的{}预热账号",
                warmup_scope.label()
            ))
        })
}

fn is_warmup_scope_executable(
    account: &Account,
    warmup_scope: WarmupScope,
    now_timestamp: i64,
) -> bool {
    match warmup_scope {
        WarmupScope::Period => {
            is_usage_window_executable(account.codex_usage_5h.as_ref(), now_timestamp)
                && is_usage_window_executable(account.codex_usage_week.as_ref(), now_timestamp)
        }
    }
}

fn validate_warmup_scope_executable(
    account: &Account,
    warmup_scope: WarmupScope,
    now_timestamp: i64,
) -> Result<(), AppError> {
    if !is_warmup_scope_executable(account, warmup_scope, now_timestamp) {
        return Err(AppError::InvalidInput(format!(
            "该账号当前没有可执行的{}预热额度",
            warmup_scope.label()
        )));
    }

    Ok(())
}

fn is_usage_window_executable(
    usage: Option<&crate::account::CodexUsageWindow>,
    _now_timestamp: i64,
) -> bool {
    let Some(usage) = usage else {
        return false;
    };

    let used_percent = if usage.used_percent.is_finite() {
        usage.used_percent.clamp(0.0, 100.0)
    } else {
        0.0
    };
    let remaining_percent = 100.0 - used_percent;

    remaining_percent > PERIOD_WARMUP_REMAINING_QUOTA_EPSILON
}

fn resolve_warmup_workspace(
    usage_repo: &UsageRepository,
    account_id: &str,
) -> Result<WarmupWorkspaceSelection, AppError> {
    if let Some(working_directory) = usage_repo.find_recent_working_directory(account_id)? {
        return Ok(WarmupWorkspaceSelection {
            working_directory: Some(working_directory),
            source: WarmupWorkingDirectorySource::RecentSession,
        });
    }

    // trusted project 只是兜底增强项；配置缺失或损坏时仍保留原有预热能力，
    // 但会把目录来源标记为 process_cwd，避免误判成真实工作区预热。
    if let Some(working_directory) = read_existing_trusted_project_paths()
        .ok()
        .and_then(|paths| paths.into_iter().next())
    {
        return Ok(WarmupWorkspaceSelection {
            working_directory: Some(working_directory),
            source: WarmupWorkingDirectorySource::TrustedProject,
        });
    }

    Ok(WarmupWorkspaceSelection {
        working_directory: None,
        source: WarmupWorkingDirectorySource::ProcessCwd,
    })
}

#[tauri::command]
pub async fn open_codex_interactive_session(
    app: AppHandle,
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
            started_at: started_at.clone(),
            completed_at: None,
            exit_code: None,
            usage_event_count: 0,
            error_message: Some("等待从 Codex 会话日志导入 Token 用量".to_string()),
        })?;
    }
    schedule_session_usage_import(
        app,
        state.inner().db.clone(),
        input.account_id.clone(),
        UsageImportSession {
            id: session_id.clone(),
            account_id: input.account_id.clone(),
            launch_mode: "interactive_terminal".to_string(),
            started_at: started_at.clone(),
        },
    );

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
    app: AppHandle,
    state: State<'_, AppState>,
    input: CodexCliLaunchInput,
) -> Result<Value, AppError> {
    let target = CodexCommandTarget::discover()?;
    let session_id = Uuid::new_v4().to_string();
    let started_at = Utc::now().to_rfc3339();
    let selected_account = prepare_launch_account(&state, input.account_id.as_deref()).await?;
    open_codex_cli_terminal(&target, &input)?;

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
        schedule_session_usage_import(
            app,
            state.inner().db.clone(),
            account.id.clone(),
            UsageImportSession {
                id: session_id.clone(),
                account_id: account.id.clone(),
                launch_mode: "cli_terminal".to_string(),
                started_at: started_at.clone(),
            },
        );
    }

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
    app: AppHandle,
    state: State<'_, AppState>,
    input: CodexAppLaunchInput,
) -> Result<Value, AppError> {
    let session_id = Uuid::new_v4().to_string();
    let started_at = Utc::now().to_rfc3339();
    let selected_account = prepare_launch_account(&state, input.account_id.as_deref()).await?;
    open_codex_desktop_app()?;

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
            started_at: started_at.clone(),
            completed_at: None,
            exit_code: None,
            usage_event_count: 0,
            error_message: Some("等待从 Codex 会话日志导入 Token 用量".to_string()),
        })?;
        schedule_session_usage_import(
            app,
            state.inner().db.clone(),
            account.id.clone(),
            UsageImportSession {
                id: session_id.clone(),
                account_id: account.id.clone(),
                launch_mode: "codex_app".to_string(),
                started_at: started_at.clone(),
            },
        );
    }

    Ok(serde_json::to_value(CodexLaunchResult {
        session_id,
        status: "launched".to_string(),
        exit_code: None,
        usage_event_count: 0,
        message: "已启动 Codex App".to_string(),
        stderr_preview: None,
    })?)
}

#[tauri::command]
pub async fn close_codex_app() -> Result<Value, AppError> {
    Ok(serde_json::to_value(close_codex_desktop_app()?)?)
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

fn schedule_session_usage_import(
    app: AppHandle,
    db: Arc<Mutex<Database>>,
    account_id: String,
    session: UsageImportSession,
) {
    tauri::async_runtime::spawn(async move {
        let mut last_error_message = None;

        for attempt in 1..=SESSION_USAGE_IMPORT_ATTEMPTS {
            if attempt > 1 {
                tokio::time::sleep(StdDuration::from_secs(
                    SESSION_USAGE_IMPORT_INTERVAL_SECONDS,
                ))
                .await;
            }

            let import_result = {
                let db = db.lock().await;
                let usage_repo = UsageRepository::new(&db);
                if let Err(error) = usage_repo.update_launch_session_usage_import_status(
                    &session.id,
                    "usage_importing",
                    Some("正在从 Codex 会话日志导入 Token 用量"),
                ) {
                    log::warn!("更新会话用量补采状态失败: {error}");
                }
                import_codex_session_usage_for_session(&usage_repo, &account_id, session.clone())
                    .and_then(|result| {
                        let usage_event_count =
                            usage_repo.get_launch_session_usage_event_count(&session.id)?;
                        Ok((result, usage_event_count))
                    })
            };

            match import_result {
                Ok((result, usage_event_count)) if usage_event_count > 0 => {
                    {
                        let db = db.lock().await;
                        let usage_repo = UsageRepository::new(&db);
                        if let Err(error) = usage_repo.update_launch_session_usage_import_status(
                            &session.id,
                            "completed",
                            None,
                        ) {
                            log::warn!("标记会话用量补采完成失败: {error}");
                        }
                    }
                    let _ = app.emit(
                        "codex-session-usage-imported",
                        serde_json::json!({
                            "account_id": account_id.clone(),
                            "session_id": session.id.clone(),
                            "status": "completed",
                            "imported_count": result.imported_count,
                            "usage_event_count": usage_event_count,
                            "scanned_file_count": result.scanned_file_count,
                            "ignored_line_count": result.ignored_line_count,
                        }),
                    );
                    return;
                }
                Ok((result, _usage_event_count)) => {
                    last_error_message = Some(format!(
                        "第 {attempt}/{SESSION_USAGE_IMPORT_ATTEMPTS} 次补采未发现可导入 Token 用量，已扫描 {} 个日志文件",
                        result.scanned_file_count
                    ));
                }
                Err(error) => {
                    last_error_message = Some(format!(
                        "第 {attempt}/{SESSION_USAGE_IMPORT_ATTEMPTS} 次补采失败: {error}"
                    ));
                }
            }

            let status = if attempt == SESSION_USAGE_IMPORT_ATTEMPTS {
                "usage_import_failed"
            } else {
                "usage_import_waiting"
            };
            let message = last_error_message.as_deref();
            let db = db.lock().await;
            let usage_repo = UsageRepository::new(&db);
            if let Err(error) =
                usage_repo.update_launch_session_usage_import_status(&session.id, status, message)
            {
                log::warn!("记录会话用量补采失败路径失败: {error}");
            }
        }

        let message =
            last_error_message.unwrap_or_else(|| "会话用量补采未产生可导入结果".to_string());
        let _ = app.emit(
            "codex-session-usage-imported",
            serde_json::json!({
                "account_id": account_id,
                "session_id": session.id,
                "status": "failed",
                "message": message,
                "imported_count": 0,
                "scanned_file_count": 0,
                "ignored_line_count": 0,
            }),
        );
    });
}

async fn refresh_quota_after_codex_task(app: &AppHandle, state: &AppState, account_id: &str) {
    let (account, credential) =
        match status_sync::load_account_and_credential(state, account_id).await {
            Ok(pair) => pair,
            Err(_) => return,
        };
    let outcome = match status_sync::evaluate_account_refresh(&account, &credential).await {
        Ok(outcome) => outcome,
        Err(_) => return,
    };
    let refreshed_account = {
        let db = state.db.lock().await;
        let repo = AccountRepository::new(&db);
        match status_sync::persist_refresh_outcome(&repo, account_id, &outcome) {
            Ok(account) => account,
            Err(_) => return,
        }
    };

    let _ = app.emit(
        "account-status-updated",
        serde_json::json!({
            "account_id": refreshed_account.id.clone(),
            "status": refreshed_account.status.to_string(),
            "message": refreshed_account.status_message.clone(),
            "account": refreshed_account.clone(),
        }),
    );
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::{AccountStatus, AuthType, CodexUsageWindow, CreateAccountInput};
    use crate::auth::PendingOAuthLogin;
    use std::path::PathBuf;
    use std::sync::{Arc, RwLock};
    use tokio::sync::Mutex;

    fn account_with_windows(
        five_hour: Option<CodexUsageWindow>,
        one_week: Option<CodexUsageWindow>,
    ) -> Account {
        Account {
            id: "account-1".to_string(),
            name: "周期预热账号".to_string(),
            auth_type: AuthType::OAuthToken,
            email: Some("user@example.com".to_string()),
            organization: None,
            is_default: false,
            is_active: true,
            created_at: "2026-05-16T00:00:00Z".to_string(),
            updated_at: "2026-05-16T00:00:00Z".to_string(),
            last_checked_at: None,
            status: AccountStatus::Normal,
            status_message: None,
            color: "#18a058".to_string(),
            avatar_text: None,
            codex_plan_type: None,
            codex_usage_fetched_at: None,
            codex_usage_5h: five_hour,
            codex_usage_week: one_week,
            codex_usage_error: None,
        }
    }

    fn create_api_key_account(repo: &AccountRepository, name: &str, api_key: &str) -> Account {
        repo.create(CreateAccountInput {
            name: Some(name.to_string()),
            auth_type: "api_key".to_string(),
            email: None,
            organization: None,
            color: None,
            credential_value: api_key.to_string(),
            credential_type: None,
        })
        .unwrap()
    }

    fn unique_temp_dir() -> PathBuf {
        let suffix = format!("{}-{}", std::process::id(), Uuid::new_v4().as_simple());
        std::env::temp_dir().join(format!("codexx-warmup-restore-tests-{suffix}"))
    }

    fn usage_window(used_percent: f64, window_seconds: i64, reset_at: i64) -> CodexUsageWindow {
        CodexUsageWindow {
            used_percent,
            window_seconds,
            reset_at: Some(reset_at),
        }
    }

    #[test]
    fn period_warmup_does_not_require_full_remaining_quota() {
        let now_timestamp = 1_800_000_000;
        let account = account_with_windows(
            Some(usage_window(47.5, 5 * 60 * 60, now_timestamp + 60)),
            Some(usage_window(82.0, 7 * 24 * 60 * 60, now_timestamp + 60)),
        );

        assert!(is_warmup_scope_executable(
            &account,
            WarmupScope::Period,
            now_timestamp
        ));
    }

    #[test]
    fn period_warmup_requires_both_period_windows_with_remaining_quota() {
        let now_timestamp = 1_800_000_000;
        let account = account_with_windows(
            Some(usage_window(0.0, 5 * 60 * 60, now_timestamp + 60)),
            None,
        );

        assert!(!is_warmup_scope_executable(
            &account,
            WarmupScope::Period,
            now_timestamp
        ));
    }

    #[test]
    fn period_warmup_rejects_zero_remaining_quota() {
        let now_timestamp = 1_800_000_000;
        let account = account_with_windows(
            Some(usage_window(100.0, 5 * 60 * 60, now_timestamp + 60)),
            Some(usage_window(0.0, 7 * 24 * 60 * 60, now_timestamp + 60)),
        );

        assert!(!is_warmup_scope_executable(
            &account,
            WarmupScope::Period,
            now_timestamp
        ));
    }

    #[tokio::test]
    async fn warmup_restore_writes_previous_default_account_back() {
        std::env::set_var(
            "CODEX_MANAGER_MASTER_KEY",
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        );
        let temp_dir = unique_temp_dir();
        std::fs::create_dir_all(&temp_dir).unwrap();
        std::env::set_var("CODEX_HOME", &temp_dir);

        let db = Database::new(temp_dir.join("codexX.db")).unwrap();
        let previous_account_id;
        let warmup_account_id;
        {
            let repo = AccountRepository::new(&db);
            let previous_account =
                create_api_key_account(&repo, "原默认账号", "previous-default-api-key");
            let warmup_account =
                create_api_key_account(&repo, "预热账号", "warmup-account-api-key");
            previous_account_id = previous_account.id.clone();
            warmup_account_id = warmup_account.id.clone();
            LocalAuthSyncService::write_account_to_default_auth_file(&repo, &warmup_account_id)
                .unwrap();
        }

        let state = AppState {
            db: Arc::new(Mutex::new(db)),
            window_close_action: Arc::new(RwLock::new("tray".to_string())),
            oauth_flow_lock: Arc::new(Mutex::new(())),
            pending_oauth_login: Mutex::new(None::<PendingOAuthLogin>),
            oauth_listener: Mutex::new(None),
        };

        restore_warmup_default_account(&state, Some(&previous_account_id), &warmup_account_id)
            .await
            .unwrap();

        {
            let db = state.db.lock().await;
            let repo = AccountRepository::new(&db);
            assert!(repo.get_by_id(&previous_account_id).unwrap().is_default);
            assert!(!repo.get_by_id(&warmup_account_id).unwrap().is_default);
        }

        let auth_text = std::fs::read_to_string(temp_dir.join("auth.json")).unwrap();
        assert!(auth_text.contains("previous-default-api-key"));
        assert!(!auth_text.contains("warmup-account-api-key"));

        let _ = std::fs::remove_dir_all(temp_dir);
    }
}
