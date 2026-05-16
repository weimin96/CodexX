use crate::error::{AppError, AppResult};
use crate::storage::Database;
use chrono::{DateTime, Datelike, Duration, FixedOffset, Local, NaiveDate, Offset, TimeZone, Utc};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::path::Path;

const MIN_TIMEZONE_OFFSET_MINUTES: i32 = -14 * 60;
const MAX_TIMEZONE_OFFSET_MINUTES: i32 = 14 * 60;

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
    pub total_cached_input_tokens: i64,
    pub total_output_tokens: i64,
    pub total_requests: i64,
    pub total_cost: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartDataPoint {
    pub date: String,
    pub input_tokens: i64,
    pub cached_input_tokens: i64,
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
    pub period: String,
    // 前端显式传入本机偏移，避免按 UTC 截断 completed_at 时把本地今日统计到昨天。
    pub timezone_offset_minutes: Option<i32>,
}

#[derive(Debug, Clone)]
struct UsageDateScope {
    period: String,
    offset: FixedOffset,
    start_date: NaiveDate,
    end_date: NaiveDate,
}

#[derive(Debug, Clone)]
struct ApiUsageEventSummary {
    completed_at: String,
    input_tokens: i64,
    cached_input_tokens: i64,
    output_tokens: i64,
    estimated_cost: f64,
}

#[derive(Debug, Clone, Default)]
struct UsagePointAccumulator {
    input_tokens: i64,
    cached_input_tokens: i64,
    output_tokens: i64,
    request_count: i64,
    cost: f64,
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

    pub fn delete_api_usage_event(&self, event_id: &str) -> AppResult<bool> {
        let changed = self.db.get_conn().execute(
            "DELETE FROM api_usage_events WHERE id = ?1",
            params![event_id],
        )?;
        Ok(changed > 0)
    }

    pub fn begin_rebuild_transaction(&self) -> AppResult<()> {
        self.db
            .get_conn()
            .execute_batch("BEGIN IMMEDIATE TRANSACTION")?;
        Ok(())
    }

    pub fn commit_rebuild_transaction(&self) -> AppResult<()> {
        self.db.get_conn().execute_batch("COMMIT")?;
        Ok(())
    }

    pub fn rollback_rebuild_transaction(&self) -> AppResult<()> {
        self.db.get_conn().execute_batch("ROLLBACK")?;
        Ok(())
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

    pub fn replace_launch_session_usage_count(
        &self,
        session_id: &str,
        usage_event_count: i64,
    ) -> AppResult<()> {
        let normalized_count = usage_event_count.max(0);
        self.db.get_conn().execute(
            "UPDATE codex_launch_sessions
             SET usage_event_count = ?2,
                 status = CASE
                    WHEN ?2 > 0 AND status = 'launched' THEN 'completed'
                    ELSE status
                 END,
                 completed_at = CASE
                    WHEN ?2 > 0 THEN COALESCE(completed_at, ?3)
                    ELSE completed_at
                 END
             WHERE id = ?1",
            params![session_id, normalized_count, Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    pub fn update_launch_session_usage_import_status(
        &self,
        session_id: &str,
        status: &str,
        error_message: Option<&str>,
    ) -> AppResult<()> {
        self.db.get_conn().execute(
            "UPDATE codex_launch_sessions
             SET status = ?2,
                 error_message = ?3
             WHERE id = ?1",
            params![session_id, status, error_message],
        )?;
        Ok(())
    }

    pub fn get_launch_session_usage_event_count(&self, session_id: &str) -> AppResult<i64> {
        let usage_event_count = self.db.get_conn().query_row(
            "SELECT usage_event_count
             FROM codex_launch_sessions
             WHERE id = ?1",
            params![session_id],
            |row| row.get::<_, i64>(0),
        )?;
        Ok(usage_event_count)
    }

    pub fn reconcile_session_log_usage_events(
        &self,
        session_id: &str,
        retained_event_ids: &HashSet<String>,
    ) -> AppResult<usize> {
        let existing_event_ids = {
            let mut stmt = self.db.get_conn().prepare(
                "SELECT id
                 FROM api_usage_events
                 WHERE session_id = ?1
                   AND source IN (
                    'codex_interactive_terminal_session_log',
                    'codex_cli_terminal_session_log',
                    'codex_codex_app_session_log'
                   )",
            )?;
            let event_ids = stmt
                .query_map(params![session_id], |row| row.get::<_, String>(0))?
                .collect::<Result<Vec<_>, _>>()?;
            event_ids
        };

        let mut deleted_count = 0;
        for event_id in existing_event_ids {
            if retained_event_ids.contains(&event_id) {
                continue;
            }

            if self.delete_api_usage_event(&event_id)? {
                deleted_count += 1;
            }
        }

        Ok(deleted_count)
    }

    pub fn delete_session_log_usage_events_for_account_sessions(
        &self,
        account_id: &str,
        session_ids: &[String],
    ) -> AppResult<usize> {
        let mut deleted_count = 0;
        for session_id in session_ids {
            let changed = self.db.get_conn().execute(
                "DELETE FROM api_usage_events
                 WHERE account_id = ?1
                   AND session_id = ?2
                   AND source IN (
                    'codex_interactive_terminal_session_log',
                    'codex_cli_terminal_session_log',
                    'codex_codex_app_session_log'
                   )",
                params![account_id, session_id],
            )?;
            deleted_count += changed;
        }
        Ok(deleted_count)
    }

    pub fn account_exists(&self, account_id: &str) -> AppResult<bool> {
        let exists = self
            .db
            .get_conn()
            .query_row(
                "SELECT 1 FROM accounts WHERE id = ?1 LIMIT 1",
                params![account_id],
                |_| Ok(()),
            )
            .optional()?
            .is_some();
        Ok(exists)
    }

    pub fn find_session_log_owner_id(
        &self,
        file_started_at: DateTime<Utc>,
        ownership_backtrack: Duration,
        ownership_forward: Duration,
    ) -> AppResult<Option<String>> {
        let earliest_started_at = (file_started_at - ownership_forward).to_rfc3339();
        let latest_started_at = (file_started_at + ownership_backtrack).to_rfc3339();
        let owner_id = self
            .db
            .get_conn()
            .query_row(
                "SELECT id
             FROM codex_launch_sessions
             WHERE started_at >= ?1
               AND started_at <= ?2
               AND launch_mode IN ('interactive_terminal', 'cli_terminal', 'codex_app')
               AND status <> 'failed'
             ORDER BY started_at DESC
             LIMIT 1",
                params![earliest_started_at, latest_started_at],
                |row| row.get::<_, String>(0),
            )
            .optional()?;

        Ok(owner_id)
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

    pub fn list_session_log_rebuild_candidates(
        &self,
        account_id: &str,
    ) -> AppResult<Vec<UsageImportSession>> {
        self.list_session_log_candidates(account_id, None, None)
    }

    pub fn find_recent_session_log_rebuild_candidate(
        &self,
        account_id: &str,
    ) -> AppResult<Option<UsageImportSession>> {
        Ok(self
            .list_session_log_candidates(account_id, None, Some(1))?
            .into_iter()
            .next())
    }

    fn list_session_log_candidates(
        &self,
        account_id: &str,
        since: Option<String>,
        limit: Option<usize>,
    ) -> AppResult<Vec<UsageImportSession>> {
        let mut sql = String::from(
            "SELECT id, account_id, launch_mode, started_at
             FROM codex_launch_sessions
             WHERE account_id = ?1
               AND launch_mode IN ('interactive_terminal', 'cli_terminal', 'codex_app')
               AND status <> 'failed'",
        );
        if since.is_some() {
            sql.push_str(" AND started_at >= ?2");
        }
        sql.push_str(" ORDER BY started_at DESC");
        if let Some(limit) = limit {
            sql.push_str(&format!(" LIMIT {}", limit.max(1)));
        }

        let mut stmt = self.db.get_conn().prepare(&sql)?;
        let map_row = |row: &rusqlite::Row<'_>| {
            Ok(UsageImportSession {
                id: row.get(0)?,
                account_id: row.get(1)?,
                launch_mode: row.get(2)?,
                started_at: row.get(3)?,
            })
        };
        let sessions = if let Some(since) = since {
            stmt.query_map(params![account_id, since], map_row)?
                .collect::<Result<Vec<_>, _>>()?
        } else {
            stmt.query_map(params![account_id], map_row)?
                .collect::<Result<Vec<_>, _>>()?
        };

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

    pub fn get_summary(&self, query: &UsageQuery) -> AppResult<UsageSummary> {
        let scope = UsageDateScope::from_query(query)?;
        let points = self.collect_usage_points(&query.account_id, &scope)?;
        let total = points.iter().fold(
            UsagePointAccumulator::default(),
            |mut accumulator, point| {
                accumulator.input_tokens += point.input_tokens;
                accumulator.cached_input_tokens += point.cached_input_tokens;
                accumulator.output_tokens += point.output_tokens;
                accumulator.request_count += point.request_count;
                accumulator.cost += point.cost;
                accumulator
            },
        );

        Ok(UsageSummary {
            account_id: query.account_id.clone(),
            period: scope.period,
            total_input_tokens: total.input_tokens,
            total_cached_input_tokens: total.cached_input_tokens,
            total_output_tokens: total.output_tokens,
            total_requests: total.request_count,
            total_cost: total.cost,
        })
    }

    pub fn get_chart_data(&self, query: &UsageQuery) -> AppResult<Vec<ChartDataPoint>> {
        let scope = UsageDateScope::from_query(query)?;
        self.collect_usage_points(&query.account_id, &scope)
    }

    fn collect_usage_points(
        &self,
        account_id: &str,
        scope: &UsageDateScope,
    ) -> AppResult<Vec<ChartDataPoint>> {
        let mut point_map = BTreeMap::new();
        self.add_usage_record_points(account_id, scope, &mut point_map)?;
        self.add_api_usage_event_points(account_id, scope, &mut point_map)?;
        Ok(point_map
            .into_iter()
            .map(|(date, accumulator)| ChartDataPoint {
                date,
                input_tokens: accumulator.input_tokens,
                cached_input_tokens: accumulator.cached_input_tokens,
                output_tokens: accumulator.output_tokens,
                request_count: accumulator.request_count,
                cost: accumulator.cost,
            })
            .collect())
    }

    fn add_usage_record_points(
        &self,
        account_id: &str,
        scope: &UsageDateScope,
        point_map: &mut BTreeMap<String, UsagePointAccumulator>,
    ) -> AppResult<()> {
        let conn = self.db.get_conn();
        let mut stmt = conn.prepare(
            "SELECT date,
                    SUM(input_tokens),
                    SUM(output_tokens),
                    SUM(request_count),
                    SUM(estimated_cost)
             FROM usage_records
             WHERE account_id = ?1
               AND date BETWEEN ?2 AND ?3
             GROUP BY date
             ORDER BY date ASC",
        )?;
        let rows = stmt
            .query_map(
                params![account_id, scope.start_date_key(), scope.end_date_key()],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, i64>(3)?,
                        row.get::<_, f64>(4)?,
                    ))
                },
            )?
            .collect::<Result<Vec<_>, _>>()?;

        for (date, input_tokens, output_tokens, request_count, cost) in rows {
            let accumulator = point_map.entry(date).or_default();
            accumulator.input_tokens += input_tokens;
            accumulator.output_tokens += output_tokens;
            accumulator.request_count += request_count;
            accumulator.cost += cost;
        }

        Ok(())
    }

    fn add_api_usage_event_points(
        &self,
        account_id: &str,
        scope: &UsageDateScope,
        point_map: &mut BTreeMap<String, UsagePointAccumulator>,
    ) -> AppResult<()> {
        for event in self.list_api_usage_events_since(account_id, scope)? {
            let local_date = scope.completed_at_date(&event.completed_at)?;
            if !scope.contains_date(local_date) {
                continue;
            }

            let accumulator = point_map.entry(format_date_key(local_date)).or_default();
            accumulator.input_tokens += event.input_tokens;
            accumulator.cached_input_tokens += event.cached_input_tokens;
            accumulator.output_tokens += event.output_tokens;
            accumulator.request_count += 1;
            accumulator.cost += event.estimated_cost;
        }

        Ok(())
    }

    fn list_api_usage_events_since(
        &self,
        account_id: &str,
        scope: &UsageDateScope,
    ) -> AppResult<Vec<ApiUsageEventSummary>> {
        let conn = self.db.get_conn();
        let mut stmt = conn.prepare(
            "SELECT completed_at, input_tokens, COALESCE(cached_input_tokens, 0), output_tokens, estimated_cost
             FROM api_usage_events
             WHERE account_id = ?1
               AND completed_at >= ?2",
        )?;
        let events = stmt
            .query_map(params![account_id, scope.start_utc_key()?], |row| {
                Ok(ApiUsageEventSummary {
                    completed_at: row.get(0)?,
                    input_tokens: row.get(1)?,
                    cached_input_tokens: row.get(2)?,
                    output_tokens: row.get(3)?,
                    estimated_cost: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(events)
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
        let today = Local::now().date_naive();
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

impl UsageDateScope {
    fn from_query(query: &UsageQuery) -> AppResult<Self> {
        let offset = resolve_timezone_offset(query.timezone_offset_minutes)?;
        let end_date = Utc::now().with_timezone(&offset).date_naive();
        let start_date = resolve_period_start_date(&query.period, end_date)?;

        Ok(Self {
            period: query.period.clone(),
            offset,
            start_date,
            end_date,
        })
    }

    fn start_date_key(&self) -> String {
        format_date_key(self.start_date)
    }

    fn end_date_key(&self) -> String {
        format_date_key(self.end_date)
    }

    fn start_utc_key(&self) -> AppResult<String> {
        let start_time = self
            .start_date
            .and_hms_opt(0, 0, 0)
            .ok_or_else(|| AppError::Other("构造用量统计开始时间失败".to_string()))?;
        let local_start = self
            .offset
            .from_local_datetime(&start_time)
            .single()
            .ok_or_else(|| AppError::Other("转换用量统计开始时间失败".to_string()))?;
        Ok(local_start.with_timezone(&Utc).to_rfc3339())
    }

    fn completed_at_date(&self, completed_at: &str) -> AppResult<NaiveDate> {
        completed_at_to_date(completed_at, self.offset)
    }

    fn contains_date(&self, date: NaiveDate) -> bool {
        date >= self.start_date && date <= self.end_date
    }
}

fn resolve_period_start_date(period: &str, end_date: NaiveDate) -> AppResult<NaiveDate> {
    match period {
        "day" => Ok(end_date),
        "week" => Ok(end_date - Duration::days(6)),
        "month" => Ok(end_date - Duration::days(29)),
        "year" => Ok(end_date - Duration::days(364)),
        "current_month" => NaiveDate::from_ymd_opt(end_date.year(), end_date.month(), 1)
            .ok_or_else(|| AppError::Other("构造本月用量统计开始日期失败".to_string())),
        "current_year" => NaiveDate::from_ymd_opt(end_date.year(), 1, 1)
            .ok_or_else(|| AppError::Other("构造今年用量统计开始日期失败".to_string())),
        other => Err(AppError::InvalidInput(format!(
            "不支持的用量统计周期: {other}"
        ))),
    }
}

fn resolve_timezone_offset(timezone_offset_minutes: Option<i32>) -> AppResult<FixedOffset> {
    let offset_minutes = timezone_offset_minutes.unwrap_or_else(default_timezone_offset_minutes);
    if !(MIN_TIMEZONE_OFFSET_MINUTES..=MAX_TIMEZONE_OFFSET_MINUTES).contains(&offset_minutes) {
        return Err(AppError::InvalidInput(format!(
            "时区偏移超出支持范围: {offset_minutes} 分钟"
        )));
    }

    let offset_seconds = offset_minutes
        .checked_mul(60)
        .ok_or_else(|| AppError::InvalidInput("时区偏移换算失败".to_string()))?;
    FixedOffset::east_opt(offset_seconds)
        .ok_or_else(|| AppError::InvalidInput("时区偏移不合法".to_string()))
}

fn default_timezone_offset_minutes() -> i32 {
    Local::now().offset().fix().local_minus_utc() / 60
}

fn completed_at_to_date(completed_at: &str, offset: FixedOffset) -> AppResult<NaiveDate> {
    let completed_at = DateTime::parse_from_rfc3339(completed_at)
        .map_err(|error| AppError::Other(format!("解析用量完成时间失败: {error}")))?;
    Ok(completed_at.with_timezone(&offset).date_naive())
}

fn format_date_key(date: NaiveDate) -> String {
    date.format("%Y-%m-%d").to_string()
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
    use super::{
        format_date_key, normalize_existing_working_directory, ApiUsageEventRecord,
        CodexLaunchSessionRecord, UsageDateScope, UsageQuery, UsageRepository,
    };
    use crate::storage::Database;
    use chrono::{Datelike, Duration, FixedOffset, TimeZone, Utc};
    use rusqlite::params;
    use std::collections::HashSet;
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

        let db = Database::new(temp_dir.join("codexX.db")).unwrap();
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

        let db = Database::new(temp_dir.join("codexX.db")).unwrap();
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

    #[test]
    fn year_period_covers_recent_365_days_for_query_timezone() {
        let offset_minutes = 8 * 60;
        let offset = FixedOffset::east_opt(offset_minutes * 60).unwrap();
        let today = Utc::now().with_timezone(&offset).date_naive();
        let query = UsageQuery {
            account_id: "account-a".to_string(),
            period: "year".to_string(),
            timezone_offset_minutes: Some(offset_minutes),
        };
        let scope = UsageDateScope::from_query(&query).unwrap();

        assert_eq!(scope.end_date, today);
        assert_eq!(scope.start_date, today - Duration::days(364));
    }

    #[test]
    fn current_periods_start_at_calendar_boundaries() {
        let offset_minutes = 8 * 60;
        let offset = FixedOffset::east_opt(offset_minutes * 60).unwrap();
        let today = Utc::now().with_timezone(&offset).date_naive();

        let current_month_scope = UsageDateScope::from_query(&UsageQuery {
            account_id: "account-a".to_string(),
            period: "current_month".to_string(),
            timezone_offset_minutes: Some(offset_minutes),
        })
        .unwrap();
        let current_year_scope = UsageDateScope::from_query(&UsageQuery {
            account_id: "account-a".to_string(),
            period: "current_year".to_string(),
            timezone_offset_minutes: Some(offset_minutes),
        })
        .unwrap();

        assert_eq!(
            current_month_scope.start_date,
            chrono::NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap()
        );
        assert_eq!(
            current_year_scope.start_date,
            chrono::NaiveDate::from_ymd_opt(today.year(), 1, 1).unwrap()
        );
    }

    #[test]
    fn chart_data_groups_api_events_by_query_timezone_date() {
        let isolated_data_dir = unique_temp_dir();
        std::fs::create_dir_all(&isolated_data_dir).unwrap();

        let db = Database::new(isolated_data_dir.join("codexX.db")).unwrap();
        seed_account(&db, "account-a");
        let repo = UsageRepository::new(&db);
        let offset_minutes = 8 * 60;
        let offset = FixedOffset::east_opt(offset_minutes * 60).unwrap();
        let local_today = Utc::now().with_timezone(&offset).date_naive();
        let local_event_time = offset
            .with_ymd_and_hms(
                local_today.year(),
                local_today.month(),
                local_today.day(),
                0,
                30,
                0,
            )
            .single()
            .unwrap();
        let completed_at = local_event_time.with_timezone(&Utc).to_rfc3339();

        repo.insert_api_usage_event(&ApiUsageEventRecord {
            id: "event-local-today".to_string(),
            account_id: "account-a".to_string(),
            session_id: None,
            source: "usage_regression".to_string(),
            endpoint: None,
            model: None,
            response_id: None,
            request_id: None,
            status_code: None,
            input_tokens: 12,
            output_tokens: 5,
            total_tokens: 17,
            cached_input_tokens: Some(3),
            reasoning_tokens: None,
            estimated_cost: 0.0,
            raw_usage_json: None,
            is_complete: true,
            error_message: None,
            started_at: completed_at.clone(),
            completed_at,
            created_at: Utc::now().to_rfc3339(),
        })
        .unwrap();

        let query = UsageQuery {
            account_id: "account-a".to_string(),
            period: "day".to_string(),
            timezone_offset_minutes: Some(offset_minutes),
        };
        let points = repo.get_chart_data(&query).unwrap();
        let summary = repo.get_summary(&query).unwrap();

        assert_eq!(points.len(), 1);
        assert_eq!(points[0].date, format_date_key(local_today));
        assert_eq!(points[0].input_tokens, 12);
        assert_eq!(points[0].cached_input_tokens, 3);
        assert_eq!(points[0].output_tokens, 5);
        assert_eq!(points[0].request_count, 1);
        assert_eq!(summary.total_input_tokens, 12);
        assert_eq!(summary.total_cached_input_tokens, 3);
        assert_eq!(summary.total_output_tokens, 5);
        assert_eq!(summary.total_requests, 1);
        let _ = std::fs::remove_dir_all(isolated_data_dir);
    }

    #[test]
    fn reconciliation_removes_session_log_events_not_retained() {
        let isolated_data_dir = unique_temp_dir();
        std::fs::create_dir_all(&isolated_data_dir).unwrap();

        let db = Database::new(isolated_data_dir.join("codexX.db")).unwrap();
        seed_account(&db, "account-a");
        let repo = UsageRepository::new(&db);
        repo.insert_launch_session(&CodexLaunchSessionRecord {
            id: "session-account-a".to_string(),
            account_id: "account-a".to_string(),
            launch_mode: "cli_terminal".to_string(),
            executable: None,
            working_directory: None,
            prompt_preview: None,
            status: "completed".to_string(),
            started_at: "2026-04-26T08:00:00Z".to_string(),
            completed_at: Some("2026-04-26T08:10:00Z".to_string()),
            exit_code: None,
            usage_event_count: 1,
            error_message: None,
        })
        .unwrap();
        repo.insert_api_usage_event(&ApiUsageEventRecord {
            id: "misattributed-session-log-event".to_string(),
            account_id: "account-a".to_string(),
            session_id: Some("session-account-a".to_string()),
            source: "codex_cli_terminal_session_log".to_string(),
            endpoint: Some("~/.codex/sessions".to_string()),
            model: None,
            response_id: None,
            request_id: None,
            status_code: None,
            input_tokens: 20,
            output_tokens: 5,
            total_tokens: 25,
            cached_input_tokens: Some(0),
            reasoning_tokens: None,
            estimated_cost: 0.0,
            raw_usage_json: None,
            is_complete: true,
            error_message: None,
            started_at: "2026-04-26T08:00:00Z".to_string(),
            completed_at: Utc::now().to_rfc3339(),
            created_at: Utc::now().to_rfc3339(),
        })
        .unwrap();

        let deleted_count = repo
            .reconcile_session_log_usage_events("session-account-a", &HashSet::new())
            .unwrap();
        repo.replace_launch_session_usage_count("session-account-a", 0)
            .unwrap();
        let summary = repo
            .get_summary(&UsageQuery {
                account_id: "account-a".to_string(),
                period: "day".to_string(),
                timezone_offset_minutes: Some(8 * 60),
            })
            .unwrap();

        assert_eq!(deleted_count, 1);
        assert_eq!(summary.total_input_tokens, 0);
        assert_eq!(summary.total_output_tokens, 0);
        let _ = std::fs::remove_dir_all(isolated_data_dir);
    }

    #[test]
    fn account_rebuild_delete_preserves_other_account_session_log_events() {
        let isolated_data_dir = unique_temp_dir();
        std::fs::create_dir_all(&isolated_data_dir).unwrap();

        let db = Database::new(isolated_data_dir.join("codexX.db")).unwrap();
        seed_account(&db, "account-a");
        seed_account(&db, "account-b");
        let repo = UsageRepository::new(&db);
        seed_completed_launch_session(&repo, "session-a", "account-a");
        seed_completed_launch_session(&repo, "session-b", "account-b");
        seed_session_log_event(&repo, "event-a", "account-a", "session-a", 12);
        seed_session_log_event(&repo, "event-b", "account-b", "session-b", 30);

        let deleted_count = repo
            .delete_session_log_usage_events_for_account_sessions(
                "account-a",
                &["session-a".to_string()],
            )
            .unwrap();

        let account_a_summary = repo
            .get_summary(&UsageQuery {
                account_id: "account-a".to_string(),
                period: "day".to_string(),
                timezone_offset_minutes: Some(8 * 60),
            })
            .unwrap();
        let account_b_summary = repo
            .get_summary(&UsageQuery {
                account_id: "account-b".to_string(),
                period: "day".to_string(),
                timezone_offset_minutes: Some(8 * 60),
            })
            .unwrap();

        assert_eq!(deleted_count, 1);
        assert_eq!(account_a_summary.total_input_tokens, 0);
        assert_eq!(account_b_summary.total_input_tokens, 30);
        let _ = std::fs::remove_dir_all(isolated_data_dir);
    }

    #[test]
    fn launch_session_usage_import_status_records_failure_message() {
        let isolated_data_dir = unique_temp_dir();
        std::fs::create_dir_all(&isolated_data_dir).unwrap();

        let db = Database::new(isolated_data_dir.join("codexX.db")).unwrap();
        seed_account(&db, "account-a");
        let repo = UsageRepository::new(&db);
        seed_completed_launch_session(&repo, "session-a", "account-a");

        repo.update_launch_session_usage_import_status(
            "session-a",
            "usage_import_failed",
            Some("未发现可导入 Token 用量"),
        )
        .unwrap();

        let (status, error_message) = db
            .get_conn()
            .query_row(
                "SELECT status, error_message
                 FROM codex_launch_sessions
                 WHERE id = ?1",
                params!["session-a"],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)),
            )
            .unwrap();

        assert_eq!(status, "usage_import_failed");
        assert_eq!(error_message.as_deref(), Some("未发现可导入 Token 用量"));
        let _ = std::fs::remove_dir_all(isolated_data_dir);
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

    fn seed_completed_launch_session(repo: &UsageRepository, session_id: &str, account_id: &str) {
        repo.insert_launch_session(&CodexLaunchSessionRecord {
            id: session_id.to_string(),
            account_id: account_id.to_string(),
            launch_mode: "cli_terminal".to_string(),
            executable: None,
            working_directory: None,
            prompt_preview: None,
            status: "completed".to_string(),
            started_at: "2026-04-26T08:00:00Z".to_string(),
            completed_at: Some("2026-04-26T08:10:00Z".to_string()),
            exit_code: None,
            usage_event_count: 1,
            error_message: None,
        })
        .unwrap();
    }

    fn seed_session_log_event(
        repo: &UsageRepository,
        event_id: &str,
        account_id: &str,
        session_id: &str,
        input_tokens: i64,
    ) {
        repo.insert_api_usage_event(&ApiUsageEventRecord {
            id: event_id.to_string(),
            account_id: account_id.to_string(),
            session_id: Some(session_id.to_string()),
            source: "codex_cli_terminal_session_log".to_string(),
            endpoint: Some("~/.codex/sessions".to_string()),
            model: None,
            response_id: None,
            request_id: None,
            status_code: None,
            input_tokens,
            output_tokens: 0,
            total_tokens: input_tokens,
            cached_input_tokens: Some(0),
            reasoning_tokens: None,
            estimated_cost: 0.0,
            raw_usage_json: None,
            is_complete: true,
            error_message: None,
            started_at: "2026-04-26T08:00:00Z".to_string(),
            completed_at: Utc::now().to_rfc3339(),
            created_at: Utc::now().to_rfc3339(),
        })
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
