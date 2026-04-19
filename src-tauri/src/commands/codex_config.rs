use serde_json::Value;

use crate::codex_config::{read_user_codex_config, save_user_codex_config};
use crate::error::AppError;

#[tauri::command]
pub async fn read_codex_config_file() -> Result<Value, AppError> {
    Ok(serde_json::to_value(read_user_codex_config()?)?)
}

#[tauri::command]
pub async fn save_codex_config_file(raw_text: String) -> Result<Value, AppError> {
    Ok(serde_json::to_value(save_user_codex_config(&raw_text)?)?)
}
