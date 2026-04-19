use crate::codex_runtime::resolve_codex_home;
use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
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

#[derive(Debug, Deserialize)]
pub struct CodexConfigFieldUpdate {
    pub key: String,
    pub value: String,
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

pub fn save_user_codex_config_field(
    input: CodexConfigFieldUpdate,
) -> AppResult<CodexConfigSnapshot> {
    validate_config_key(&input.key)?;
    validate_config_value(&input.value)?;

    let current_snapshot = read_user_codex_config()?;
    let next_text = replace_or_insert_config_field(
        &current_snapshot.raw_text,
        input.key.trim(),
        input.value.trim(),
    );
    let normalized_text = normalize_config_text(&next_text);
    validate_config_text(&normalized_text)?;

    let config_path = user_config_path()?;
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

fn validate_config_key(key: &str) -> AppResult<()> {
    let normalized = key.trim();
    if normalized.is_empty() {
        return Err(AppError::InvalidInput("配置字段名不能为空".to_string()));
    }

    validate_config_text(&format!("{normalized} = true\n"))
}

fn validate_config_value(value: &str) -> AppResult<()> {
    let normalized = value.trim();
    if normalized.is_empty() {
        return Err(AppError::InvalidInput("配置字段值不能为空".to_string()));
    }

    validate_config_text(&format!("__codex_manager_value__ = {normalized}\n"))
}

fn replace_or_insert_config_field(raw_text: &str, key: &str, value: &str) -> String {
    let mut lines = raw_text
        .lines()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let key_parts = key.split('.').collect::<Vec<_>>();
    let leaf_key = key_parts.last().copied().unwrap_or(key);
    let parent_table = if key_parts.len() > 1 {
        Some(key_parts[..key_parts.len() - 1].join("."))
    } else {
        None
    };
    let mut current_table = String::new();
    let mut parent_table_insert_index = None;

    for index in 0..lines.len() {
        let trimmed = lines[index].trim();
        if let Some(table_name) = parse_table_header(trimmed) {
            current_table = table_name.to_string();
            continue;
        }

        if parent_table.as_deref() == Some(current_table.as_str()) {
            parent_table_insert_index = Some(index + 1);
        }

        let Some(assignment_key) = parse_assignment_key(trimmed) else {
            continue;
        };
        let matches_root_key = current_table.is_empty() && assignment_key == key;
        let matches_dotted_key = assignment_key == key;
        let matches_table_leaf =
            parent_table.as_deref() == Some(current_table.as_str()) && assignment_key == leaf_key;
        if matches_root_key || matches_dotted_key || matches_table_leaf {
            lines[index] = replace_assignment_value(&lines[index], value);
            return join_config_lines(lines);
        }
    }

    let new_line = if parent_table_insert_index.is_some() {
        format!("{leaf_key} = {value}")
    } else {
        format!("{key} = {value}")
    };

    if let Some(index) = parent_table_insert_index {
        lines.insert(index, new_line);
    } else {
        if !lines.is_empty() && lines.last().is_some_and(|line| !line.trim().is_empty()) {
            lines.push(String::new());
        }
        lines.push(new_line);
    }

    join_config_lines(lines)
}

fn parse_table_header(line: &str) -> Option<&str> {
    if line.starts_with("[[") || !line.starts_with('[') || !line.ends_with(']') {
        return None;
    }

    line.strip_prefix('[')?.strip_suffix(']').map(str::trim)
}

fn parse_assignment_key(line: &str) -> Option<&str> {
    if line.is_empty() || line.starts_with('#') || line.starts_with('[') {
        return None;
    }

    line.split_once('=')
        .map(|(key, _)| key.trim())
        .filter(|key| !key.is_empty())
}

fn replace_assignment_value(line: &str, value: &str) -> String {
    let Some((left, right)) = line.split_once('=') else {
        return line.to_string();
    };
    let indent = left.len() - left.trim_start().len();
    let key = left.trim();
    let comment = right
        .find('#')
        .map(|index| format!(" {}", right[index..].trim()))
        .unwrap_or_default();

    format!("{}{} = {}{}", " ".repeat(indent), key, value, comment)
}

fn join_config_lines(lines: Vec<String>) -> String {
    if lines.is_empty() {
        String::new()
    } else {
        format!("{}\n", lines.join("\n"))
    }
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

    #[test]
    fn updates_existing_field_without_rewriting_other_lines() {
        let next_text = replace_or_insert_config_field(
            r#"# 保留注释
model = "gpt-5.2" # 当前模型
[sandbox_workspace_write]
network_access = false
"#,
            "model",
            "\"gpt-5.4\"",
        );

        assert!(next_text.contains("# 保留注释"));
        assert!(next_text.contains("model = \"gpt-5.4\" # 当前模型"));
        assert!(next_text.contains("network_access = false"));
    }

    #[test]
    fn inserts_nested_field_inside_existing_table() {
        let next_text = replace_or_insert_config_field(
            r#"[sandbox_workspace_write]
writable_roots = ["C:\\work"]
"#,
            "sandbox_workspace_write.network_access",
            "true",
        );

        assert!(next_text.contains("[sandbox_workspace_write]\nwritable_roots"));
        assert!(next_text.contains("network_access = true"));
    }
}
