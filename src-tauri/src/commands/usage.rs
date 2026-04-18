use serde_json::Value;
use tauri::State;

use crate::error::AppError;
use crate::usage::{UsageQuery, UsageRepository};
use crate::AppState;

#[tauri::command]
pub async fn fetch_usage(
    state: State<'_, AppState>,
    account_id: String,
) -> Result<(), AppError> {
    // In a real app, this would fetch from the OpenAI API
    // For now, ensure demo data is seeded
    let db = state.db.lock().await;
    let repo = UsageRepository::new(&db);
    repo.seed_demo_data(&account_id)?;
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
