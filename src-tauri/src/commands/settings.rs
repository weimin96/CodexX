use rusqlite::params;
use serde_json::Value;
use tauri::State;
use tauri_plugin_autostart::ManagerExt as _;

use crate::error::AppError;
use crate::AppState;

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<Value, AppError> {
    let db = state.db.lock().await;
    let conn = db.get_conn();
    let mut stmt = conn.prepare("SELECT key, value FROM settings")?;
    let pairs: Vec<(String, String)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<Result<Vec<_>, _>>()?;

    let mut map = serde_json::Map::new();
    for (k, v) in pairs {
        map.insert(k, Value::String(v));
    }

    // Defaults
    if !map.contains_key("theme") {
        map.insert("theme".to_string(), Value::String("light".to_string()));
    }
    if !map.contains_key("language") {
        map.insert("language".to_string(), Value::String("zh-CN".to_string()));
    }
    if !map.contains_key("check_interval") {
        map.insert(
            "check_interval".to_string(),
            Value::String("300".to_string()),
        );
    }
    if !map.contains_key("autostart") {
        map.insert("autostart".to_string(), Value::String("false".to_string()));
    }
    Ok(Value::Object(map))
}

#[tauri::command]
pub async fn save_settings(state: State<'_, AppState>, settings: Value) -> Result<(), AppError> {
    let db = state.db.lock().await;
    let conn = db.get_conn();
    let now = chrono::Utc::now().to_rfc3339();

    if let Some(obj) = settings.as_object() {
        for (key, value) in obj {
            let val_str = match value {
                Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            conn.execute(
                "INSERT INTO settings (key, value, updated_at) VALUES (?1, ?2, ?3)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
                params![key, val_str, now],
            )?;
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn set_autostart(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<(), AppError> {
    if enabled {
        app.autolaunch()
            .enable()
            .map_err(|err| AppError::Other(format!("设置开机自启失败: {}", err)))?;
    } else {
        app.autolaunch()
            .disable()
            .map_err(|err| AppError::Other(format!("关闭开机自启失败: {}", err)))?;
    }

    let db = state.db.lock().await;
    let conn = db.get_conn();
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO settings (key, value, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
        params!["autostart", enabled.to_string(), now],
    )?;

    Ok(())
}
