use serde_json::Value;
use tauri::State;

use crate::codex_session_import::{
    import_codex_session_usage_for_account, rebuild_codex_session_usage_for_account,
    CodexSessionUsageRebuildScope,
};
use crate::error::AppError;
use crate::usage::{UsageQuery, UsageRepository};
use crate::AppState;

#[tauri::command]
pub async fn fetch_usage(
    state: State<'_, AppState>,
    account_id: String,
) -> Result<Value, AppError> {
    let db = state.db.lock().await;
    let repo = UsageRepository::new(&db);
    let import_result = import_codex_session_usage_for_account(&repo, &account_id)?;
    Ok(serde_json::to_value(import_result)?)
}

#[tauri::command]
pub async fn get_usage_stats(
    state: State<'_, AppState>,
    query: UsageQuery,
) -> Result<Value, AppError> {
    let db = state.db.lock().await;
    let repo = UsageRepository::new(&db);
    let summary = repo.get_summary(&query)?;
    Ok(serde_json::to_value(summary)?)
}

#[tauri::command]
pub async fn get_usage_chart_data(
    state: State<'_, AppState>,
    query: UsageQuery,
) -> Result<Value, AppError> {
    let db = state.db.lock().await;
    let repo = UsageRepository::new(&db);
    let data = repo.get_chart_data(&query)?;
    Ok(serde_json::to_value(data)?)
}

#[tauri::command]
pub async fn clear_usage_data(state: State<'_, AppState>) -> Result<(), AppError> {
    let db = state.db.lock().await;
    let repo = UsageRepository::new(&db);
    repo.clear_all_usage()?;
    Ok(())
}

#[tauri::command]
pub async fn rebuild_account_usage(
    state: State<'_, AppState>,
    account_id: String,
    scope: String,
) -> Result<Value, AppError> {
    let db = state.db.lock().await;
    let repo = UsageRepository::new(&db);
    let rebuild_scope = CodexSessionUsageRebuildScope::parse(scope.as_str())?;
    let rebuild_result =
        rebuild_codex_session_usage_for_account(&repo, &account_id, rebuild_scope)?;
    Ok(serde_json::to_value(rebuild_result)?)
}
