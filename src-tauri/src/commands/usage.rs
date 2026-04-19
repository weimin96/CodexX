use serde_json::Value;
use tauri::State;

use crate::error::AppError;
use crate::usage::{UsageQuery, UsageRepository};
use crate::AppState;

#[tauri::command]
pub async fn fetch_usage(state: State<'_, AppState>, account_id: String) -> Result<(), AppError> {
    // 用量刷新只读取本地统计库；真实 Token 记录由受控 Codex 启动入口写入。
    let db = state.db.lock().await;
    let repo = UsageRepository::new(&db);
    let _ = repo.get_summary(&account_id, "month")?;
    Ok(())
}

#[tauri::command]
pub async fn get_usage_stats(
    state: State<'_, AppState>,
    query: UsageQuery,
) -> Result<Value, AppError> {
    let db = state.db.lock().await;
    let repo = UsageRepository::new(&db);
    let summary = repo.get_summary(&query.account_id, &query.period)?;
    Ok(serde_json::to_value(summary)?)
}

#[tauri::command]
pub async fn get_usage_chart_data(
    state: State<'_, AppState>,
    query: UsageQuery,
) -> Result<Value, AppError> {
    let db = state.db.lock().await;
    let repo = UsageRepository::new(&db);
    let data = repo.get_chart_data(&query.account_id, &query.period)?;
    Ok(serde_json::to_value(data)?)
}

#[tauri::command]
pub async fn clear_usage_data(state: State<'_, AppState>) -> Result<(), AppError> {
    let db = state.db.lock().await;
    let repo = UsageRepository::new(&db);
    repo.clear_all_usage()?;
    Ok(())
}
