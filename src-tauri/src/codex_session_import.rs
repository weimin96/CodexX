use crate::codex_runtime::resolve_codex_home;
use crate::codex_token_usage::{
    is_codex_token_count_event, parse_codex_token_count_usage, CodexTokenCountState,
    CodexTokenUsageMetrics,
};
use crate::error::{AppError, AppResult};
use crate::usage::{ApiUsageEventRecord, UsageImportSession, UsageRepository};
use chrono::{DateTime, Duration, TimeZone, Utc};
use serde::Serialize;
use serde_json::{Map, Value};
use std::collections::HashSet;
use std::fs::File;
use std::hash::Hasher;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

const LEGACY_TOKEN_COUNT_ITEM_SCAN_LIMIT: usize = 4;
// Codex 会话日志会在整个对话过程中持续改写；用修改时间归属会让旧账号扫描到新账号日志。
// 归属窗口基于日志创建或命名时间，并用最近一次启动记录确定账号边界。
const SESSION_LOG_DISCOVERY_BACKTRACK_MINUTES: i64 = 5;
const SESSION_LOG_DISCOVERY_FORWARD_HOURS: i64 = 24;

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
        let candidate_files = collect_candidate_session_files(usage_repo, &sessions_dir, session)?;
        let mut imported_for_session = 0;
        let mut retained_event_ids = HashSet::new();
        for file_path in candidate_files {
            scanned_files.insert(file_path.clone());
            let import_result = import_session_file(usage_repo, account_id, session, &file_path)?;
            imported_for_session += import_result.imported_count;
            ignored_line_count += import_result.ignored_line_count;
            retained_event_ids.extend(import_result.retained_event_ids);
        }

        usage_repo.reconcile_session_log_usage_events(&session.id, &retained_event_ids)?;
        usage_repo
            .replace_launch_session_usage_count(&session.id, retained_event_ids.len() as i64)?;
        imported_count += imported_for_session;
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
    retained_event_ids: Vec<String>,
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
    let mut retained_event_ids = Vec::new();
    let mut token_count_state = CodexTokenCountState::default();

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
        let parsed_items = parse_usage_items(&value, &mut token_count_state);
        if is_codex_token_count_event(&value) {
            delete_legacy_token_count_items(
                usage_repo,
                &session.id,
                file_path,
                line_index,
                parsed_items.len(),
            )?;
        }

        for (item_index, parsed_item) in parsed_items.into_iter().enumerate() {
            let event_id =
                build_session_usage_event_id(&session.id, file_path, line_index, item_index);
            retained_event_ids.push(event_id.clone());
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
        retained_event_ids,
    })
}

fn delete_legacy_token_count_items(
    usage_repo: &UsageRepository<'_>,
    session_id: &str,
    file_path: &Path,
    line_index: usize,
    retained_item_count: usize,
) -> AppResult<()> {
    // 旧算法会把 last_token_usage 与 total_token_usage 作为同一行里的多个候选写入。
    // 新算法只保留增量候选，刷新时删除同源多余候选，避免历史统计继续膨胀。
    for item_index in retained_item_count..LEGACY_TOKEN_COUNT_ITEM_SCAN_LIMIT {
        let event_id = build_session_usage_event_id(session_id, file_path, line_index, item_index);
        usage_repo.delete_api_usage_event(&event_id)?;
    }
    Ok(())
}

fn collect_candidate_session_files(
    usage_repo: &UsageRepository<'_>,
    sessions_dir: &Path,
    session: &UsageImportSession,
) -> AppResult<Vec<PathBuf>> {
    let started_at = DateTime::parse_from_rfc3339(&session.started_at)
        .map_err(|error| AppError::Other(format!("解析 Codex 启动时间失败: {error}")))?
        .with_timezone(&Utc);
    let min_file_started_at =
        started_at - Duration::minutes(SESSION_LOG_DISCOVERY_BACKTRACK_MINUTES);
    let max_file_started_at = started_at + Duration::hours(SESSION_LOG_DISCOVERY_FORWARD_HOURS);
    let mut files = Vec::new();
    collect_owned_jsonl_files(
        usage_repo,
        sessions_dir,
        session,
        min_file_started_at,
        max_file_started_at,
        &mut files,
    )?;
    files.sort();
    Ok(files)
}

fn collect_owned_jsonl_files(
    usage_repo: &UsageRepository<'_>,
    directory: &Path,
    session: &UsageImportSession,
    min_file_started_at: DateTime<Utc>,
    max_file_started_at: DateTime<Utc>,
    files: &mut Vec<PathBuf>,
) -> AppResult<()> {
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        let metadata = entry.metadata()?;
        if metadata.is_dir() {
            collect_owned_jsonl_files(
                usage_repo,
                &path,
                session,
                min_file_started_at,
                max_file_started_at,
                files,
            )?;
            continue;
        }

        if path.extension().and_then(|extension| extension.to_str()) != Some("jsonl") {
            continue;
        }

        let Some(file_started_at) = resolve_session_file_started_at(&path, &metadata) else {
            continue;
        };
        if file_started_at < min_file_started_at || file_started_at > max_file_started_at {
            continue;
        }

        let owner_id = usage_repo.find_session_log_owner_id(
            file_started_at,
            Duration::minutes(SESSION_LOG_DISCOVERY_BACKTRACK_MINUTES),
            Duration::hours(SESSION_LOG_DISCOVERY_FORWARD_HOURS),
        )?;
        if owner_id.as_deref() == Some(session.id.as_str()) {
            files.push(path);
        }
    }
    Ok(())
}

fn resolve_session_file_started_at(
    file_path: &Path,
    metadata: &std::fs::Metadata,
) -> Option<DateTime<Utc>> {
    parse_session_file_timestamp(file_path)
        .or_else(|| metadata.created().ok().map(DateTime::<Utc>::from))
        .or_else(|| metadata.modified().ok().map(DateTime::<Utc>::from))
}

fn parse_session_file_timestamp(file_path: &Path) -> Option<DateTime<Utc>> {
    let file_name = file_path.file_name()?.to_str()?;
    let bytes = file_name.as_bytes();
    if bytes.len() < 19 {
        return None;
    }

    for start in 0..=(bytes.len() - 19) {
        let Some(timestamp) = parse_timestamp_at(bytes, start) else {
            continue;
        };
        return Some(timestamp);
    }

    None
}

fn parse_timestamp_at(bytes: &[u8], start: usize) -> Option<DateTime<Utc>> {
    if bytes.get(start + 4) != Some(&b'-')
        || bytes.get(start + 7) != Some(&b'-')
        || !matches!(bytes.get(start + 10), Some(b'T') | Some(b'_'))
        || !matches!(bytes.get(start + 13), Some(b'-') | Some(b':'))
        || !matches!(bytes.get(start + 16), Some(b'-') | Some(b':'))
    {
        return None;
    }

    let year = parse_digits(bytes, start, 4)? as i32;
    let month = parse_digits(bytes, start + 5, 2)?;
    let day = parse_digits(bytes, start + 8, 2)?;
    let hour = parse_digits(bytes, start + 11, 2)?;
    let minute = parse_digits(bytes, start + 14, 2)?;
    let second = parse_digits(bytes, start + 17, 2)?;

    Utc.with_ymd_and_hms(year, month, day, hour, minute, second)
        .single()
}

fn parse_digits(bytes: &[u8], start: usize, length: usize) -> Option<u32> {
    let mut value = 0;
    for index in start..start + length {
        let byte = *bytes.get(index)?;
        if !byte.is_ascii_digit() {
            return None;
        }
        value = value * 10 + u32::from(byte - b'0');
    }
    Some(value)
}

fn read_event_timestamp(value: &Value) -> Option<DateTime<Utc>> {
    let timestamp = value.get("timestamp").and_then(Value::as_str)?;
    DateTime::parse_from_rfc3339(timestamp)
        .ok()
        .map(|value| value.with_timezone(&Utc))
}

fn parse_usage_items(
    root: &Value,
    token_count_state: &mut CodexTokenCountState,
) -> Vec<ParsedSessionUsage> {
    if let Some(metrics) = parse_codex_token_count_usage(root, token_count_state) {
        return vec![ParsedSessionUsage::from_codex_token_count(root, metrics)];
    }

    if is_codex_token_count_event(root) {
        return Vec::new();
    }

    let mut items = Vec::new();
    collect_usage_candidates(root, root, &mut items);

    let mut seen = HashSet::new();
    items
        .into_iter()
        .filter(|item| seen.insert(item.raw_usage_json.clone()))
        .collect()
}

impl ParsedSessionUsage {
    fn from_codex_token_count(root: &Value, metrics: CodexTokenUsageMetrics) -> Self {
        Self {
            model: find_string(root, &["model", "model_name"]),
            response_id: find_string(root, &["response_id", "responseId", "id"]),
            request_id: find_string(root, &["request_id", "requestId", "x_request_id"]),
            input_tokens: metrics.input_tokens,
            output_tokens: metrics.output_tokens,
            total_tokens: metrics.total_tokens,
            cached_input_tokens: metrics.cached_input_tokens,
            reasoning_tokens: metrics.reasoning_tokens,
            raw_usage_json: metrics.raw_usage_json,
        }
    }
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
    use crate::storage::Database;
    use crate::usage::CodexLaunchSessionRecord;
    use rusqlite::params;
    use serde_json::json;
    use std::time::{SystemTime, UNIX_EPOCH};

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

        let mut token_count_state = CodexTokenCountState::default();
        let items = parse_usage_items(&value, &mut token_count_state);

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
    fn parses_codex_rollout_file_timestamp() {
        let file_path = Path::new(
            r"C:\Users\pwm\.codex\sessions\2026\04\26\rollout-2026-04-26T08-10-00-000Z.jsonl",
        );

        let timestamp = parse_session_file_timestamp(file_path).unwrap();

        assert_eq!(timestamp.to_rfc3339(), "2026-04-26T08:10:00+00:00");
    }

    #[test]
    fn candidate_session_files_use_nearest_launch_owner() {
        let isolated_data_dir = unique_isolated_data_dir();
        let sessions_dir = isolated_data_dir.join("sessions");
        std::fs::create_dir_all(&sessions_dir).unwrap();

        let db = Database::new(isolated_data_dir.join("codexX.db")).unwrap();
        seed_import_account(&db, "account-a");
        seed_import_account(&db, "account-b");
        let repo = UsageRepository::new(&db);
        let session_a = UsageImportSession {
            id: "session-account-a".to_string(),
            account_id: "account-a".to_string(),
            launch_mode: "cli_terminal".to_string(),
            started_at: "2026-04-26T08:00:00Z".to_string(),
        };
        let session_b = UsageImportSession {
            id: "session-account-b".to_string(),
            account_id: "account-b".to_string(),
            launch_mode: "cli_terminal".to_string(),
            started_at: "2026-04-26T08:09:00Z".to_string(),
        };
        seed_launch_session(&repo, &session_a);
        seed_launch_session(&repo, &session_b);

        let session_file = sessions_dir.join("rollout-2026-04-26T08-10-00-000Z.jsonl");
        std::fs::write(
            &session_file,
            r#"{"timestamp":"2026-04-26T08:10:30Z","usage":{"input_tokens":8,"output_tokens":2}}"#,
        )
        .unwrap();

        let account_a_files =
            collect_candidate_session_files(&repo, &sessions_dir, &session_a).unwrap();
        let account_b_files =
            collect_candidate_session_files(&repo, &sessions_dir, &session_b).unwrap();

        assert!(account_a_files.is_empty());
        assert_eq!(account_b_files, vec![session_file]);
        let _ = std::fs::remove_dir_all(isolated_data_dir);
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

        let mut token_count_state = CodexTokenCountState::default();
        let items = parse_usage_items(&value, &mut token_count_state);

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].total_tokens, 17);
    }

    #[test]
    fn parses_codex_token_count_from_last_usage_only() {
        let value = json!({
            "timestamp": "2026-04-19T00:00:00Z",
            "type": "event_msg",
            "payload": {
                "type": "token_count",
                "info": {
                    "model": "gpt-5.4",
                    "total_token_usage": {
                        "input_tokens": 100,
                        "cached_input_tokens": 20,
                        "output_tokens": 30,
                        "reasoning_output_tokens": 5
                    },
                    "last_token_usage": {
                        "input_tokens": 12,
                        "cached_input_tokens": 3,
                        "output_tokens": 4,
                        "reasoning_output_tokens": 1
                    }
                }
            }
        });
        let mut token_count_state = CodexTokenCountState::default();

        let items = parse_usage_items(&value, &mut token_count_state);

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].model.as_deref(), Some("gpt-5.4"));
        assert_eq!(items[0].input_tokens, 12);
        assert_eq!(items[0].output_tokens, 4);
        assert_eq!(items[0].total_tokens, 16);
        assert_eq!(items[0].cached_input_tokens, Some(3));
        assert_eq!(items[0].reasoning_tokens, Some(1));
    }

    #[test]
    fn skips_repeated_codex_token_count_snapshot() {
        let value = json!({
            "timestamp": "2026-04-19T00:00:00Z",
            "type": "event_msg",
            "payload": {
                "type": "token_count",
                "info": {
                    "total_token_usage": {
                        "input_tokens": 100,
                        "cached_input_tokens": 20,
                        "output_tokens": 30
                    },
                    "last_token_usage": {
                        "input_tokens": 100,
                        "cached_input_tokens": 20,
                        "output_tokens": 30
                    }
                }
            }
        });
        let mut token_count_state = CodexTokenCountState::default();

        assert_eq!(parse_usage_items(&value, &mut token_count_state).len(), 1);
        assert!(parse_usage_items(&value, &mut token_count_state).is_empty());
    }

    fn seed_import_account(db: &Database, account_id: &str) {
        db.get_conn()
            .execute(
                "INSERT INTO accounts (
                    id, name, auth_type, is_default, is_active, created_at, updated_at, status, color
                 ) VALUES (?1, ?2, 'oauth_token', 0, 1, ?3, ?3, 'unknown', '#18a058')",
                params![account_id, "导入测试账号", "2026-04-26T08:00:00Z"],
            )
            .unwrap();
    }

    fn seed_launch_session(repo: &UsageRepository, session: &UsageImportSession) {
        repo.insert_launch_session(&CodexLaunchSessionRecord {
            id: session.id.clone(),
            account_id: session.account_id.clone(),
            launch_mode: session.launch_mode.clone(),
            executable: None,
            working_directory: None,
            prompt_preview: None,
            status: "launched".to_string(),
            started_at: session.started_at.clone(),
            completed_at: None,
            exit_code: None,
            usage_event_count: 0,
            error_message: None,
        })
        .unwrap();
    }

    fn unique_isolated_data_dir() -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("codexx-session-import-tests-{suffix}"))
    }
}
