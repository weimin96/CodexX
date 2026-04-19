use crate::codex_runtime::resolve_codex_home;
use crate::error::{AppError, AppResult};
use serde::Serialize;
use std::path::{Path, PathBuf};
use toml::Value as TomlValue;

#[derive(Debug, Serialize)]
pub struct CodexConfigEntry {
    pub key: String,
    pub value_type: String,
    pub value: String,
}

#[derive(Debug, Serialize)]
pub struct CodexConfigSnapshot {
    pub path: String,
    pub exists: bool,
    pub raw_text: String,
    pub parsed_entries: Vec<CodexConfigEntry>,
    pub backup_path: Option<String>,
}

pub fn read_user_codex_config() -> AppResult<CodexConfigSnapshot> {
    let config_path = user_config_path()?;
    let exists = config_path.exists();
    let raw_text = if exists {
        std::fs::read_to_string(&config_path)?
    } else {
        String::new()
    };

    build_snapshot(config_path, exists, raw_text, None)
}

pub fn save_user_codex_config(raw_text: &str) -> AppResult<CodexConfigSnapshot> {
    let config_path = user_config_path()?;
    let normalized_text = normalize_config_text(raw_text);
    validate_config_text(&normalized_text)?;
    let backup_path = write_config_with_backup(&config_path, &normalized_text)?;

    build_snapshot(
        config_path,
        true,
        normalized_text,
        backup_path.map(|path| path.to_string_lossy().to_string()),
    )
}

fn user_config_path() -> AppResult<PathBuf> {
    Ok(resolve_codex_home()?.join("config.toml"))
}

fn build_snapshot(
    config_path: PathBuf,
    exists: bool,
    raw_text: String,
    backup_path: Option<String>,
) -> AppResult<CodexConfigSnapshot> {
    let parsed_entries = parse_config_entries(&raw_text)?;

    Ok(CodexConfigSnapshot {
        path: config_path.to_string_lossy().to_string(),
        exists,
        raw_text,
        parsed_entries,
        backup_path,
    })
}

fn parse_config_entries(raw_text: &str) -> AppResult<Vec<CodexConfigEntry>> {
    let parsed_config = parse_config_document(raw_text)?;
    let mut entries = Vec::new();
    collect_config_entries("", &parsed_config, &mut entries);
    entries.sort_by(|first, second| first.key.cmp(&second.key));
    Ok(entries)
}

fn parse_config_document(raw_text: &str) -> AppResult<TomlValue> {
    if raw_text.trim().is_empty() {
        return Ok(TomlValue::Table(toml::map::Map::new()));
    }

    toml::from_str::<TomlValue>(raw_text)
        .map_err(|error| AppError::InvalidInput(format!("Codex config.toml 格式无效: {error}")))
}

fn collect_config_entries(prefix: &str, value: &TomlValue, entries: &mut Vec<CodexConfigEntry>) {
    match value {
        TomlValue::Table(table) => {
            for (key, child_value) in table {
                let next_key = if prefix.is_empty() {
                    key.to_string()
                } else {
                    format!("{prefix}.{key}")
                };
                collect_config_entries(&next_key, child_value, entries);
            }
        }
        _ => entries.push(CodexConfigEntry {
            key: prefix.to_string(),
            value_type: config_value_type(value).to_string(),
            value: display_config_value(value),
        }),
    }
}

fn config_value_type(value: &TomlValue) -> &'static str {
    match value {
        TomlValue::String(_) => "string",
        TomlValue::Integer(_) => "number",
        TomlValue::Float(_) => "number",
        TomlValue::Boolean(_) => "boolean",
        TomlValue::Datetime(_) => "datetime",
        TomlValue::Array(_) => "array",
        TomlValue::Table(_) => "table",
    }
}

fn display_config_value(value: &TomlValue) -> String {
    match value {
        TomlValue::String(text) => text.to_string(),
        _ => value.to_string().replace('\n', " "),
    }
}

fn normalize_config_text(raw_text: &str) -> String {
    let trimmed = raw_text.trim_end_matches(|character| character == '\r' || character == '\n');
    if trimmed.is_empty() {
        String::new()
    } else {
        format!("{trimmed}\n")
    }
}

fn validate_config_text(raw_text: &str) -> AppResult<()> {
    parse_config_document(raw_text).map(|_| ())
}

fn write_config_with_backup(config_path: &Path, raw_text: &str) -> AppResult<Option<PathBuf>> {
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let temp_path = config_path.with_extension("toml.tmp");
    let backup_path = config_path.with_extension("toml.bak");
    std::fs::write(&temp_path, raw_text)?;

    let backup_path = if config_path.exists() {
        // 写入前保留一份同目录备份，避免用户手写配置在保存失败时不可恢复。
        std::fs::copy(config_path, &backup_path)?;
        Some(backup_path)
    } else {
        None
    };

    if config_path.exists() {
        std::fs::remove_file(config_path)?;
    }

    if let Err(error) = std::fs::rename(&temp_path, config_path) {
        if let Some(backup_path) = backup_path.as_ref() {
            let _ = std::fs::copy(backup_path, config_path);
        }
        let _ = std::fs::remove_file(&temp_path);
        return Err(AppError::Io(error));
    }

    Ok(backup_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nested_config_entries() {
        let entries = parse_config_entries(
            r#"
model = "gpt-5.4"
[sandbox_workspace_write]
network_access = true
writable_roots = ["C:\\work"]
"#,
        )
        .unwrap();

        let keys = entries
            .iter()
            .map(|entry| entry.key.as_str())
            .collect::<Vec<_>>();

        assert_eq!(
            keys,
            vec![
                "model",
                "sandbox_workspace_write.network_access",
                "sandbox_workspace_write.writable_roots"
            ]
        );
        assert_eq!(entries[0].value, "gpt-5.4");
        assert_eq!(entries[1].value_type, "boolean");
    }

    #[test]
    fn rejects_invalid_toml_before_saving() {
        let result = validate_config_text("model = ");

        assert!(result.is_err());
    }
}
