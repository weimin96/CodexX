use crate::error::AppResult;
use rusqlite::Connection;
use std::collections::HashSet;
use std::path::Path;

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new(path: impl AsRef<Path>) -> AppResult<Self> {
        let conn = Connection::open(path)?;
        let db = Self { conn };
        db.initialize()?;
        Ok(db)
    }

    fn initialize(&self) -> AppResult<()> {
        self.conn.execute_batch(
            "
            PRAGMA journal_mode=WAL;
            PRAGMA foreign_keys=ON;

            CREATE TABLE IF NOT EXISTS accounts (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                auth_type TEXT NOT NULL,
                email TEXT,
                organization TEXT,
                is_default INTEGER NOT NULL DEFAULT 0,
                is_active INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                last_checked_at TEXT,
                status TEXT NOT NULL DEFAULT 'unknown',
                status_message TEXT,
                color TEXT NOT NULL DEFAULT '#18a058',
                avatar_text TEXT
            );

            CREATE TABLE IF NOT EXISTS credentials (
                id TEXT PRIMARY KEY,
                account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
                credential_type TEXT NOT NULL,
                encrypted_value TEXT NOT NULL,
                expires_at TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS usage_records (
                id TEXT PRIMARY KEY,
                account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
                date TEXT NOT NULL,
                input_tokens INTEGER NOT NULL DEFAULT 0,
                output_tokens INTEGER NOT NULL DEFAULT 0,
                request_count INTEGER NOT NULL DEFAULT 0,
                estimated_cost REAL NOT NULL DEFAULT 0.0,
                model TEXT,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS codex_launch_sessions (
                id TEXT PRIMARY KEY,
                account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
                launch_mode TEXT NOT NULL,
                executable TEXT,
                working_directory TEXT,
                prompt_preview TEXT,
                status TEXT NOT NULL,
                started_at TEXT NOT NULL,
                completed_at TEXT,
                exit_code INTEGER,
                usage_event_count INTEGER NOT NULL DEFAULT 0,
                error_message TEXT
            );

            CREATE TABLE IF NOT EXISTS api_usage_events (
                id TEXT PRIMARY KEY,
                account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
                session_id TEXT REFERENCES codex_launch_sessions(id) ON DELETE SET NULL,
                source TEXT NOT NULL,
                endpoint TEXT,
                model TEXT,
                response_id TEXT,
                request_id TEXT,
                status_code INTEGER,
                input_tokens INTEGER NOT NULL DEFAULT 0,
                output_tokens INTEGER NOT NULL DEFAULT 0,
                total_tokens INTEGER NOT NULL DEFAULT 0,
                cached_input_tokens INTEGER,
                reasoning_tokens INTEGER,
                estimated_cost REAL NOT NULL DEFAULT 0.0,
                raw_usage_json TEXT,
                is_complete INTEGER NOT NULL DEFAULT 1,
                error_message TEXT,
                started_at TEXT NOT NULL,
                completed_at TEXT NOT NULL,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_usage_account_date 
                ON usage_records(account_id, date);
            CREATE INDEX IF NOT EXISTS idx_api_usage_account_completed
                ON api_usage_events(account_id, completed_at);
            CREATE INDEX IF NOT EXISTS idx_api_usage_session
                ON api_usage_events(session_id);
            CREATE INDEX IF NOT EXISTS idx_credentials_account 
                ON credentials(account_id);
        ",
        )?;
        self.ensure_account_profile_columns()?;
        Ok(())
    }

    fn ensure_account_profile_columns(&self) -> AppResult<()> {
        let mut stmt = self.conn.prepare("PRAGMA table_info(accounts)")?;
        let columns = stmt
            .query_map([], |row| row.get::<_, String>(1))?
            .collect::<Result<HashSet<_>, _>>()?;

        let required_columns = [
            ("codex_plan_type", "TEXT"),
            ("codex_usage_fetched_at", "TEXT"),
            ("codex_usage_5h_used_percent", "REAL"),
            ("codex_usage_5h_window_seconds", "INTEGER"),
            ("codex_usage_5h_reset_at", "INTEGER"),
            ("codex_usage_week_used_percent", "REAL"),
            ("codex_usage_week_window_seconds", "INTEGER"),
            ("codex_usage_week_reset_at", "INTEGER"),
            ("codex_usage_error", "TEXT"),
        ];

        for (column, definition) in required_columns {
            if !columns.contains(column) {
                self.conn.execute(
                    &format!("ALTER TABLE accounts ADD COLUMN {column} {definition}"),
                    [],
                )?;
            }
        }

        Ok(())
    }

    pub fn get_conn(&self) -> &Connection {
        &self.conn
    }
}
