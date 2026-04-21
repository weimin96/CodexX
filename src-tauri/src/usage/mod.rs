use crate::error::AppResult;
use crate::storage::Database;
use chrono::{Duration, Utc};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageRecord {
    pub id: String,
    pub account_id: String,
    pub date: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub request_count: i64,
    pub estimated_cost: f64,
    pub model: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageSummary {
    pub account_id: String,
    pub period: String,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub total_requests: i64,
    pub total_cost: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartDataPoint {
    pub date: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub request_count: i64,
    pub cost: f64,
}

#[derive(Debug, Clone)]
pub struct CodexLaunchSessionRecord {
    pub id: String,
    pub account_id: String,
    pub launch_mode: String,
    pub executable: Option<String>,
    pub working_directory: Option<String>,
    pub prompt_preview: Option<String>,
    pub status: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub exit_code: Option<i32>,
    pub usage_event_count: i64,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UsageImportSession {
    pub id: String,
    pub account_id: String,
    pub launch_mode: String,
    pub started_at: String,
}

#[derive(Debug, Clone)]
pub struct ApiUsageEventRecord {
    pub id: String,
    pub account_id: String,
    pub session_id: Option<String>,
    pub source: String,
    pub endpoint: Option<String>,
    pub model: Option<String>,
    pub response_id: Option<String>,
    pub request_id: Option<String>,
    pub status_code: Option<i32>,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub total_tokens: i64,
    pub cached_input_tokens: Option<i64>,
    pub reasoning_tokens: Option<i64>,
    pub estimated_cost: f64,
    pub raw_usage_json: Option<String>,
    pub is_complete: bool,
    pub error_message: Option<String>,
    pub started_at: String,
    pub completed_at: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct UsageQuery {
    pub account_id: String,
    pub period: String, // 可选值："day"、"week"、"month"
}

pub struct UsageRepository<'a> {
    db: &'a Database,
}

impl<'a> UsageRepository<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn upsert_usage(&self, record: &UsageRecord) -> AppResult<()> {
        let now = Utc::now().to_rfc3339();
        self.db.get_conn().execute(
            "INSERT INTO usage_records (id, account_id, date, input_tokens, output_tokens, request_count, estimated_cost, model, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(id) DO UPDATE SET
               input_tokens = excluded.input_tokens,
               output_tokens = excluded.output_tokens,
               request_count = excluded.request_count,
               estimated_cost = excluded.estimated_cost",
            params![
                record.id, record.account_id, record.date,
                record.input_tokens, record.output_tokens,
                record.request_count, record.estimated_cost,
                record.model, now
            ],
        )?;
        Ok(())
    }

    pub fn insert_launch_session(&self, record: &CodexLaunchSessionRecord) -> AppResult<()> {
        self.db.get_conn().execute(
            "INSERT INTO codex_launch_sessions (
                id, account_id, launch_mode, executable, working_directory, prompt_preview,
                status, started_at, completed_at, exit_code, usage_event_count, error_message
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
             ON CONFLICT(id) DO UPDATE SET
                status = excluded.status,
                completed_at = excluded.completed_at,
                exit_code = excluded.exit_code,
                usage_event_count = excluded.usage_event_count,
                error_message = excluded.error_message",
            params![
                &record.id,
                &record.account_id,
                &record.launch_mode,
                &record.executable,
                &record.working_directory,
                &record.prompt_preview,
                &record.status,
                &record.started_at,
                &record.completed_at,
                &record.exit_code,
                &record.usage_event_count,
                &record.error_message,
            ],
        )?;
        Ok(())
    }

    pub fn insert_api_usage_event(&self, record: &ApiUsageEventRecord) -> AppResult<bool> {
        let changed = self.db.get_conn().execute(
            "INSERT OR IGNORE INTO api_usage_events (
                id, account_id, session_id, source, endpoint, model, response_id, request_id,
                status_code, input_tokens, output_tokens, total_tokens, cached_input_tokens,
                reasoning_tokens, estimated_cost, raw_usage_json, is_complete, error_message,
                started_at, completed_at, created_at
             )
             VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14,
                ?15, ?16, ?17, ?18, ?19, ?20, ?21
             )",
            params![
                &record.id,
                &record.account_id,
                &record.session_id,
                &record.source,
                &record.endpoint,
                &record.model,
                &record.response_id,
                &record.request_id,
                &record.status_code,
                &record.input_tokens,
                &record.output_tokens,
                &record.total_tokens,
                &record.cached_input_tokens,
                &record.reasoning_tokens,
                &record.estimated_cost,
                &record.raw_usage_json,
                if record.is_complete { 1 } else { 0 },
                &record.error_message,
                &record.started_at,
                &record.completed_at,
                &record.created_at,
            ],
        )?;
        Ok(changed > 0)
    }

    pub fn add_launch_session_usage_count(
        &self,
        session_id: &str,
        imported_count: i64,
    ) -> AppResult<()> {
        if imported_count <= 0 {
            return Ok(());
        }

        self.db.get_conn().execute(
            "UPDATE codex_launch_sessions
             SET usage_event_count = usage_event_count + ?2,
                 status = CASE WHEN status = 'launched' THEN 'completed' ELSE status END,
                 completed_at = COALESCE(completed_at, ?3)
             WHERE id = ?1",
            params![session_id, imported_count, Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    pub fn list_session_log_import_candidates(
        &self,
        account_id: &str,
    ) -> AppResult<Vec<UsageImportSession>> {
        let since = (Utc::now() - Duration::days(35)).to_rfc3339();
        let mut stmt = self.db.get_conn().prepare(
            "SELECT id, account_id, launch_mode, started_at
             FROM codex_launch_sessions
             WHERE account_id = ?1
               AND started_at >= ?2
               AND launch_mode IN ('interactive_terminal', 'cli_terminal', 'codex_app')
               AND status <> 'failed'
             ORDER BY started_at DESC",
        )?;

        let sessions = stmt
            .query_map(params![account_id, since], |row| {
                Ok(UsageImportSession {
                    id: row.get(0)?,
                    account_id: row.get(1)?,
                    launch_mode: row.get(2)?,
                    started_at: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(sessions)
    }

    pub fn find_recent_working_directory(&self, account_id: &str) -> AppResult<Option<String>> {
        let mut stmt = self.db.get_conn().prepare(
            "SELECT working_directory
             FROM codex_launch_sessions
             WHERE account_id = ?1
               AND working_directory IS NOT NULL
               AND trim(working_directory) <> ''
               AND launch_mode IN ('interactive_terminal', 'cli_terminal', 'exec_json')
               AND status <> 'failed'
             ORDER BY started_at DESC
             LIMIT 20",
        )?;

        let directories = stmt
            .query_map(params![account_id], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(directories
            .into_iter()
            .find_map(|directory| normalize_existing_working_directory(&directory)))
    }

    pub fn get_summary(&self, account_id: &str, period: &str) -> AppResult<UsageSummary> {
        let (start_date, _end_date) = get_period_range(period);

        let conn = self.db.get_conn();
        let row = conn.query_row(
            "SELECT
                COALESCE(SUM(input_tokens), 0),
                COALESCE(SUM(output_tokens), 0),
                COALESCE(SUM(request_count), 0),
                COALESCE(SUM(estimated_cost), 0.0)
             FROM (
                SELECT input_tokens, output_tokens, request_count, estimated_cost
                FROM usage_records
                WHERE account_id = ?1 AND date >= ?2
                UNION ALL
                SELECT input_tokens, output_tokens, 1 AS request_count, estimated_cost
                FROM api_usage_events
                WHERE account_id = ?1 AND substr(completed_at, 1, 10) >= ?2
             )",
            params![account_id, start_date],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, f64>(3)?,
                ))
            },
        )?;

        Ok(UsageSummary {
            account_id: account_id.to_string(),
            period: period.to_string(),
            total_input_tokens: row.0,
            total_output_tokens: row.1,
            total_requests: row.2,
            total_cost: row.3,
        })
    }

    pub fn get_chart_data(&self, account_id: &str, period: &str) -> AppResult<Vec<ChartDataPoint>> {
        let (start_date, _) = get_period_range(period);

        let conn = self.db.get_conn();
        let mut stmt = conn.prepare(
            "SELECT date,
                    SUM(input_tokens),
                    SUM(output_tokens),
                    SUM(request_count),
                    SUM(estimated_cost)
             FROM (
                SELECT date, input_tokens, output_tokens, request_count, estimated_cost
                FROM usage_records
                WHERE account_id = ?1 AND date >= ?2
                UNION ALL
                SELECT substr(completed_at, 1, 10) AS date,
                       input_tokens,
                       output_tokens,
                       1 AS request_count,
                       estimated_cost
                FROM api_usage_events
                WHERE account_id = ?1 AND substr(completed_at, 1, 10) >= ?2
             )
             GROUP BY date
             ORDER BY date ASC",
        )?;

        let points = stmt
            .query_map(params![account_id, start_date], |row| {
                Ok(ChartDataPoint {
                    date: row.get(0)?,
                    input_tokens: row.get(1)?,
                    output_tokens: row.get(2)?,
                    request_count: row.get(3)?,
                    cost: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(points)
    }

    pub fn clear_all_usage(&self) -> AppResult<()> {
        let conn = self.db.get_conn();
        conn.execute("DELETE FROM api_usage_events", [])?;
        conn.execute("DELETE FROM codex_launch_sessions", [])?;
        conn.execute("DELETE FROM usage_records", [])?;
        Ok(())
    }

    /// 插入用于演示的模拟用量数据
    pub fn seed_demo_data(&self, account_id: &str) -> AppResult<()> {
        let today = Utc::now().date_naive();
        for i in 0..30 {
            let date = today - Duration::days(i);
            let date_str = date.format("%Y-%m-%d").to_string();

            let input_tokens = (rand::random::<u16>() as i64 % 50000) + 1000;
            let output_tokens = (rand::random::<u16>() as i64 % 20000) + 500;
            let requests = (rand::random::<u8>() as i64 % 100) + 5;
            let cost = (input_tokens as f64 * 0.000003) + (output_tokens as f64 * 0.000015);

            let record = UsageRecord {
                id: format!("{}-{}", account_id, date_str),
                account_id: account_id.to_string(),
                date: date_str,
                input_tokens,
                output_tokens,
                request_count: requests,
                estimated_cost: cost,
                model: Some("gpt-4o".to_string()),
                created_at: Utc::now().to_rfc3339(),
            };

            let _ = self.upsert_usage(&record);
        }
        Ok(())
    }
}

fn get_period_range(period: &str) -> (String, String) {
    let today = Utc::now().date_naive();
    let start = match period {
        "day" => today,
        "week" => today - Duration::days(6),
        "month" => today - Duration::days(29),
        _ => today - Duration::days(29),
    };
    (
        start.format("%Y-%m-%d").to_string(),
        today.format("%Y-%m-%d").to_string(),
    )
}

fn normalize_existing_working_directory(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    let path = Path::new(trimmed);
    if !path.is_dir() {
        return None;
    }

    Some(path.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::{normalize_existing_working_directory, CodexLaunchSessionRecord, UsageRepository};
    use crate::storage::Database;
    use rusqlite::params;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn prefers_latest_existing_working_directory() {
        let temp_dir = unique_temp_dir();
        std::fs::create_dir_all(&temp_dir).unwrap();
        let workspace_a = temp_dir.join("workspace-a");
        let workspace_b = temp_dir.join("workspace-b");
        std::fs::create_dir_all(&workspace_a).unwrap();
        std::fs::create_dir_all(&workspace_b).unwrap();

        let db = Database::new(temp_dir.join("codex.db")).unwrap();
        seed_account(&db, "account-a");
        let repo = UsageRepository::new(&db);

        repo.insert_launch_session(&CodexLaunchSessionRecord {
            id: "session-old".to_string(),
            account_id: "account-a".to_string(),
            launch_mode: "cli_terminal".to_string(),
            executable: None,
            working_directory: Some(workspace_a.to_string_lossy().to_string()),
            prompt_preview: None,
            status: "completed".to_string(),
            started_at: "2026-04-20T01:00:00Z".to_string(),
            completed_at: Some("2026-04-20T01:01:00Z".to_string()),
            exit_code: Some(0),
            usage_event_count: 0,
            error_message: None,
        })
        .unwrap();
        repo.insert_launch_session(&CodexLaunchSessionRecord {
            id: "session-new".to_string(),
            account_id: "account-a".to_string(),
            launch_mode: "interactive_terminal".to_string(),
            executable: None,
            working_directory: Some(workspace_b.to_string_lossy().to_string()),
            prompt_preview: None,
            status: "completed".to_string(),
            started_at: "2026-04-21T01:00:00Z".to_string(),
            completed_at: Some("2026-04-21T01:01:00Z".to_string()),
            exit_code: Some(0),
            usage_event_count: 0,
            error_message: None,
        })
        .unwrap();

        let directory = repo.find_recent_working_directory("account-a").unwrap();

        assert_eq!(directory, Some(workspace_b.to_string_lossy().to_string()));
        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn skips_missing_and_failed_working_directories() {
        let temp_dir = unique_temp_dir();
        std::fs::create_dir_all(&temp_dir).unwrap();
        let valid_workspace = temp_dir.join("valid-workspace");
        std::fs::create_dir_all(&valid_workspace).unwrap();
        let missing_workspace = temp_dir.join("missing-workspace");

        let db = Database::new(temp_dir.join("codex.db")).unwrap();
        seed_account(&db, "account-a");
        let repo = UsageRepository::new(&db);

        repo.insert_launch_session(&CodexLaunchSessionRecord {
            id: "session-failed".to_string(),
            account_id: "account-a".to_string(),
            launch_mode: "cli_terminal".to_string(),
            executable: None,
            working_directory: Some(valid_workspace.to_string_lossy().to_string()),
            prompt_preview: None,
            status: "failed".to_string(),
            started_at: "2026-04-22T01:00:00Z".to_string(),
            completed_at: Some("2026-04-22T01:01:00Z".to_string()),
            exit_code: Some(1),
            usage_event_count: 0,
            error_message: Some("failed".to_string()),
        })
        .unwrap();
        repo.insert_launch_session(&CodexLaunchSessionRecord {
            id: "session-missing".to_string(),
            account_id: "account-a".to_string(),
            launch_mode: "exec_json".to_string(),
            executable: None,
            working_directory: Some(missing_workspace.to_string_lossy().to_string()),
            prompt_preview: None,
            status: "completed".to_string(),
            started_at: "2026-04-21T01:00:00Z".to_string(),
            completed_at: Some("2026-04-21T01:01:00Z".to_string()),
            exit_code: Some(0),
            usage_event_count: 1,
            error_message: None,
        })
        .unwrap();
        repo.insert_launch_session(&CodexLaunchSessionRecord {
            id: "session-valid".to_string(),
            account_id: "account-a".to_string(),
            launch_mode: "interactive_terminal".to_string(),
            executable: None,
            working_directory: Some(valid_workspace.to_string_lossy().to_string()),
            prompt_preview: None,
            status: "completed".to_string(),
            started_at: "2026-04-20T01:00:00Z".to_string(),
            completed_at: Some("2026-04-20T01:01:00Z".to_string()),
            exit_code: Some(0),
            usage_event_count: 0,
            error_message: None,
        })
        .unwrap();

        let directory = repo.find_recent_working_directory("account-a").unwrap();

        assert_eq!(
            directory,
            Some(valid_workspace.to_string_lossy().to_string())
        );
        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn normalizes_existing_directory_only() {
        let temp_dir = unique_temp_dir();
        std::fs::create_dir_all(&temp_dir).unwrap();

        assert_eq!(
            normalize_existing_working_directory(temp_dir.to_string_lossy().as_ref()),
            Some(temp_dir.to_string_lossy().to_string())
        );
        assert_eq!(normalize_existing_working_directory(" "), None);
        assert_eq!(
            normalize_existing_working_directory(
                temp_dir.join("missing").to_string_lossy().as_ref()
            ),
            None
        );

        let _ = std::fs::remove_dir_all(temp_dir);
    }

    fn seed_account(db: &Database, account_id: &str) {
        db.get_conn()
            .execute(
                "INSERT INTO accounts (
                    id, name, auth_type, is_default, is_active, created_at, updated_at, status, color
                 ) VALUES (?1, ?2, 'oauth_token', 0, 1, ?3, ?3, 'unknown', '#18a058')",
                params![account_id, "测试账号", "2026-04-21T00:00:00Z"],
            )
            .unwrap();
    }

    fn unique_temp_dir() -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("codexx-usage-tests-{suffix}"))
    }
}
