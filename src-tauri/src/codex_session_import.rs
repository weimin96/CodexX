use crate::codex_runtime::resolve_codex_home;
use crate::error::{AppError, AppResult};
use crate::usage::{ApiUsageEventRecord, UsageImportSession, UsageRepository};
use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use serde_json::{Map, Value};
use std::collections::HashSet;
use std::fs::File;
use std::hash::Hasher;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize)]
pub struct CodexSessionUsageImportResult {
    pub account_id: String,
    pub session_count: usize,
    pub scanned_file_count: usize,
    pub imported_count: usize,
    pub ignored_line_count: usize,
}

#[derive(Debug, Clone)]
struct ParsedSessionUsage {
    model: Option<String>,
    response_id: Option<String>,
    request_id: Option<String>,
    input_tokens: i64,
    output_tokens: i64,
    total_tokens: i64,
    cached_input_tokens: Option<i64>,
    reasoning_tokens: Option<i64>,
    raw_usage_json: String,
}

pub fn import_codex_session_usage_for_account(
    usage_repo: &UsageRepository<'_>,
    account_id: &str,
) -> AppResult<CodexSessionUsageImportResult> {
    let sessions = usage_repo.list_session_log_import_candidates(account_id)?;
    let sessions_dir = resolve_codex_home()?.join("sessions");
    if !sessions_dir.exists() {
        return Ok(CodexSessionUsageImportResult {
            account_id: account_id.to_string(),
            session_count: sessions.len(),
            scanned_file_count: 0,
            imported_count: 0,
            ignored_line_count: 0,
        });
    }

    let mut scanned_files = HashSet::new();
    let mut imported_count = 0;
    let mut ignored_line_count = 0;

    for session in &sessions {
        let candidate_files = collect_candidate_session_files(&sessions_dir, &session.started_at)?;
        let mut imported_for_session = 0;
        for file_path in candidate_files {
            scanned_files.insert(file_path.clone());
            let import_result = import_session_file(usage_repo, account_id, session, &file_path)?;
            imported_for_session += import_result.imported_count;
            ignored_line_count += import_result.ignored_line_count;
        }

        if imported_for_session > 0 {
            usage_repo.add_launch_session_usage_count(&session.id, imported_for_session as i64)?;
            imported_count += imported_for_session;
        }
    }

    Ok(CodexSessionUsageImportResult {
        account_id: account_id.to_string(),
        session_count: sessions.len(),
        scanned_file_count: scanned_files.len(),
        imported_count,
        ignored_line_count,
    })
}

struct SessionFileImportResult {
    imported_count: usize,
    ignored_line_count: usize,
}

fn import_session_file(
    usage_repo: &UsageRepository<'_>,
    account_id: &str,
    session: &UsageImportSession,
    file_path: &Path,
) -> AppResult<SessionFileImportResult> {
    let file = File::open(file_path)?;
    let fallback_timestamp = file
        .metadata()
        .ok()
        .and_then(|metadata| metadata.modified().ok())
        .map(DateTime::<Utc>::from)
        .unwrap_or_else(Utc::now);
    let reader = BufReader::new(file);
    let mut imported_count = 0;
    let mut ignored_line_count = 0;

    for (line_index, line) in reader.lines().enumerate() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let Ok(value) = serde_json::from_str::<Value>(trimmed) else {
            ignored_line_count += 1;
            continue;
        };
        let event_timestamp = read_event_timestamp(&value).unwrap_or(fallback_timestamp);
        let parsed_items = parse_usage_items(&value);
        for (item_index, parsed_item) in parsed_items.into_iter().enumerate() {
            let event_id =
                build_session_usage_event_id(&session.id, file_path, line_index, item_index);
            let record = ApiUsageEventRecord {
                id: event_id,
                account_id: account_id.to_string(),
                session_id: Some(session.id.clone()),
                source: format!("codex_{}_session_log", session.launch_mode),
                endpoint: Some("~/.codex/sessions".to_string()),
                model: parsed_item.model,
                response_id: parsed_item.response_id,
                request_id: parsed_item.request_id,
                status_code: None,
                input_tokens: parsed_item.input_tokens,
                output_tokens: parsed_item.output_tokens,
                total_tokens: parsed_item.total_tokens,
                cached_input_tokens: parsed_item.cached_input_tokens,
                reasoning_tokens: parsed_item.reasoning_tokens,
                estimated_cost: 0.0,
                raw_usage_json: Some(parsed_item.raw_usage_json),
                is_complete: true,
                error_message: None,
                started_at: session.started_at.clone(),
                completed_at: event_timestamp.to_rfc3339(),
                created_at: Utc::now().to_rfc3339(),
            };
            if usage_repo.insert_api_usage_event(&record)? {
                imported_count += 1;
            }
        }
    }

    Ok(SessionFileImportResult {
        imported_count,
        ignored_line_count,
    })
}

fn collect_candidate_session_files(
    sessions_dir: &Path,
    started_at: &str,
) -> AppResult<Vec<PathBuf>> {
    let started_at = DateTime::parse_from_rfc3339(started_at)
        .map_err(|error| AppError::Other(format!("解析 Codex 启动时间失败: {error}")))?
        .with_timezone(&Utc);
    let min_modified_at: SystemTime = (started_at - Duration::minutes(5)).into();
    let mut files = Vec::new();
    collect_jsonl_files_since(sessions_dir, min_modified_at, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_jsonl_files_since(
    directory: &Path,
    min_modified_at: SystemTime,
    files: &mut Vec<PathBuf>,
) -> AppResult<()> {
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        let metadata = entry.metadata()?;
        if metadata.is_dir() {
            collect_jsonl_files_since(&path, min_modified_at, files)?;
            continue;
        }

        if path.extension().and_then(|extension| extension.to_str()) != Some("jsonl") {
            continue;
        }

        if metadata
            .modified()
            .map(|modified| modified >= min_modified_at)
            .unwrap_or(false)
        {
            files.push(path);
        }
    }
    Ok(())
}

fn read_event_timestamp(value: &Value) -> Option<DateTime<Utc>> {
    let timestamp = value.get("timestamp").and_then(Value::as_str)?;
    DateTime::parse_from_rfc3339(timestamp)
        .ok()
        .map(|value| value.with_timezone(&Utc))
}

fn parse_usage_items(root: &Value) -> Vec<ParsedSessionUsage> {
    let mut items = Vec::new();
    collect_usage_candidates(root, root, &mut items);

    let mut seen = HashSet::new();
    items
        .into_iter()
        .filter(|item| seen.insert(item.raw_usage_json.clone()))
        .collect()
}

fn collect_usage_candidates(root: &Value, value: &Value, items: &mut Vec<ParsedSessionUsage>) {
    match value {
        Value::Object(object) => {
            if let Some(usage) = object.get("usage") {
                if let Some(item) = parse_usage_candidate(root, usage) {
                    items.push(item);
                }
            } else if let Some(item) = parse_usage_candidate(root, value) {
                // 设计取舍：当对象同时存在 usage 子对象与同级 token 字段时，
                // 以 usage 子对象为准，避免同一条日志被重复计入用量。
                items.push(item);
            }

            for (key, child) in object {
                if key == "usage" {
                    continue;
                }
                collect_usage_candidates(root, child, items);
            }
        }
        Value::Array(values) => {
            for child in values {
                collect_usage_candidates(root, child, items);
            }
        }
        _ => {}
    }
}

fn parse_usage_candidate(root: &Value, usage: &Value) -> Option<ParsedSessionUsage> {
    let object = usage.as_object()?;
    let input_tokens = read_i64(object, &["input_tokens", "prompt_tokens"]).unwrap_or(0);
    let output_tokens = read_i64(object, &["output_tokens", "completion_tokens"]).unwrap_or(0);
    let total_tokens = read_i64(object, &["total_tokens"])
        .unwrap_or_else(|| input_tokens.saturating_add(output_tokens));

    if input_tokens == 0 && output_tokens == 0 && total_tokens == 0 {
        return None;
    }

    let cached_input_tokens = nested_i64(
        usage,
        &[
            &["input_tokens_details", "cached_tokens"],
            &["prompt_tokens_details", "cached_tokens"],
            &["cached_input_tokens"],
            &["cache_read_input_tokens"],
        ],
    );
    let reasoning_tokens = nested_i64(
        usage,
        &[
            &["output_tokens_details", "reasoning_tokens"],
            &["completion_tokens_details", "reasoning_tokens"],
            &["reasoning_tokens"],
            &["reasoning_output_tokens"],
        ],
    );

    Some(ParsedSessionUsage {
        model: find_string(root, &["model", "model_name"]),
        response_id: find_string(root, &["response_id", "responseId", "id"]),
        request_id: find_string(root, &["request_id", "requestId", "x_request_id"]),
        input_tokens,
        output_tokens,
        total_tokens,
        cached_input_tokens,
        reasoning_tokens,
        raw_usage_json: serde_json::to_string(usage).unwrap_or_else(|_| "{}".to_string()),
    })
}

fn read_i64(object: &Map<String, Value>, keys: &[&str]) -> Option<i64> {
    keys.iter()
        .find_map(|key| object.get(*key))
        .and_then(value_to_i64)
}

fn nested_i64(value: &Value, paths: &[&[&str]]) -> Option<i64> {
    paths.iter().find_map(|path| {
        let mut current = value;
        for key in *path {
            current = current.get(key)?;
        }
        value_to_i64(current)
    })
}

fn value_to_i64(value: &Value) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| value.as_u64().and_then(|number| i64::try_from(number).ok()))
        .or_else(|| value.as_f64().map(|number| number.round() as i64))
}

fn find_string(value: &Value, keys: &[&str]) -> Option<String> {
    match value {
        Value::Object(object) => {
            for key in keys {
                if let Some(text) = object.get(*key).and_then(Value::as_str) {
                    let trimmed = text.trim();
                    if !trimmed.is_empty() {
                        return Some(trimmed.to_string());
                    }
                }
            }

            for child in object.values() {
                if let Some(text) = find_string(child, keys) {
                    return Some(text);
                }
            }
            None
        }
        Value::Array(items) => items.iter().find_map(|item| find_string(item, keys)),
        _ => None,
    }
}

fn build_session_usage_event_id(
    session_id: &str,
    file_path: &Path,
    line_index: usize,
    item_index: usize,
) -> String {
    let source = format!(
        "{}:{}:{}:{}",
        session_id,
        file_path.to_string_lossy(),
        line_index,
        item_index
    );
    format!("codex-session-{:016x}", fnv1a64(source.as_bytes()))
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hasher = Fnv1a64::default();
    hasher.write(bytes);
    hasher.finish()
}

#[derive(Default)]
struct Fnv1a64(u64);

impl Hasher for Fnv1a64 {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        let mut hash = if self.0 == 0 {
            0xcbf2_9ce4_8422_2325
        } else {
            self.0
        };
        for byte in bytes {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        self.0 = hash;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_usage_object_without_prompt_text() {
        let value = json!({
            "timestamp": "2026-04-19T00:00:00Z",
            "payload": {
                "model": "gpt-5.4",
                "message": "不要解析字符串里的 {\"usage\":{\"input_tokens\":999}}",
                "usage": {
                    "input_tokens": 12,
                    "output_tokens": 5,
                    "total_tokens": 17,
                    "input_tokens_details": {
                        "cached_tokens": 3
                    },
                    "output_tokens_details": {
                        "reasoning_tokens": 2
                    }
                }
            }
        });

        let items = parse_usage_items(&value);

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].model.as_deref(), Some("gpt-5.4"));
        assert_eq!(items[0].input_tokens, 12);
        assert_eq!(items[0].output_tokens, 5);
        assert_eq!(items[0].total_tokens, 17);
        assert_eq!(items[0].cached_input_tokens, Some(3));
        assert_eq!(items[0].reasoning_tokens, Some(2));
    }

    #[test]
    fn builds_stable_session_event_id() {
        let file_path = Path::new(r"C:\Users\pwm\.codex\sessions\demo.jsonl");

        assert_eq!(
            build_session_usage_event_id("session-a", file_path, 7, 1),
            build_session_usage_event_id("session-a", file_path, 7, 1)
        );
    }

    #[test]
    fn avoids_double_count_when_usage_object_and_flat_tokens_coexist() {
        let value = json!({
            "timestamp": "2026-04-19T00:00:00Z",
            "payload": {
                "model": "gpt-5.4",
                "input_tokens": 12,
                "output_tokens": 5,
                "total_tokens": 17,
                "usage": {
                    "input_tokens": 12,
                    "output_tokens": 5,
                    "total_tokens": 17
                }
            }
        });

        let items = parse_usage_items(&value);

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].total_tokens, 17);
    }
}
