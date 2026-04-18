use chrono::{Duration, Utc};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use crate::error::AppResult;
use crate::storage::Database;

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

    pub fn get_summary(&self, account_id: &str, period: &str) -> AppResult<UsageSummary> {
        let (start_date, _end_date) = get_period_range(period);

        let conn = self.db.get_conn();
        let row = conn.query_row(
            "SELECT 
                COALESCE(SUM(input_tokens), 0),
                COALESCE(SUM(output_tokens), 0),
                COALESCE(SUM(request_count), 0),
                COALESCE(SUM(estimated_cost), 0.0)
             FROM usage_records
             WHERE account_id = ?1 AND date >= ?2",
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
             FROM usage_records
             WHERE account_id = ?1 AND date >= ?2
             GROUP BY date
             ORDER BY date ASC"
        )?;

        let points = stmt.query_map(params![account_id, start_date], |row| {
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
