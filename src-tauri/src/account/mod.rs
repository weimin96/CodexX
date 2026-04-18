use chrono::Utc;
use rusqlite::{params, OptionalExtension, Row};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::security;
use crate::storage::Database;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AuthType {
    ApiKey,
    OAuthToken,
    CookieSession,
    CliProfile,
}

impl std::fmt::Display for AuthType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthType::ApiKey => write!(f, "api_key"),
            AuthType::OAuthToken => write!(f, "oauth_token"),
            AuthType::CookieSession => write!(f, "cookie_session"),
            AuthType::CliProfile => write!(f, "cli_profile"),
        }
    }
}

impl TryFrom<&str> for AuthType {
    type Error = AppError;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "api_key" => Ok(AuthType::ApiKey),
            "oauth_token" => Ok(AuthType::OAuthToken),
            "cookie_session" => Ok(AuthType::CookieSession),
            "cli_profile" => Ok(AuthType::CliProfile),
            _ => Err(AppError::InvalidInput(format!("Unknown auth type: {}", s))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AccountStatus {
    Normal,
    Warning,
    Error,
    Unknown,
    Expired,
}

impl std::fmt::Display for AccountStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            AccountStatus::Normal => "normal",
            AccountStatus::Warning => "warning",
            AccountStatus::Error => "error",
            AccountStatus::Unknown => "unknown",
            AccountStatus::Expired => "expired",
        };
        write!(f, "{}", s)
    }
}

impl From<&str> for AccountStatus {
    fn from(s: &str) -> Self {
        match s {
            "normal" => AccountStatus::Normal,
            "warning" => AccountStatus::Warning,
            "error" => AccountStatus::Error,
            "expired" => AccountStatus::Expired,
            _ => AccountStatus::Unknown,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    pub name: String,
    pub auth_type: AuthType,
    pub email: Option<String>,
    pub organization: Option<String>,
    pub is_default: bool,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
    pub last_checked_at: Option<String>,
    pub status: AccountStatus,
    pub status_message: Option<String>,
    pub color: String,
    pub avatar_text: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateAccountInput {
    pub name: String,
    pub auth_type: String,
    pub email: Option<String>,
    pub organization: Option<String>,
    pub color: Option<String>,
    pub credential_value: String,
    pub credential_type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateAccountInput {
    pub id: String,
    pub name: Option<String>,
    pub email: Option<String>,
    pub organization: Option<String>,
    pub color: Option<String>,
    pub credential_value: Option<String>,
}

#[derive(Debug)]
pub struct UpsertSyncedAccountInput {
    pub stable_id: String,
    pub name: String,
    pub auth_type: AuthType,
    pub email: Option<String>,
    pub organization: Option<String>,
    pub color: Option<String>,
    pub credential_value: String,
    pub credential_type: Option<String>,
}

pub struct AccountRepository<'a> {
    db: &'a Database,
}

impl<'a> AccountRepository<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn create(&self, input: CreateAccountInput) -> AppResult<Account> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let auth_type = AuthType::try_from(input.auth_type.as_str())?;
        let color = input.color.unwrap_or_else(|| "#18a058".to_string());
        let avatar_text = input
            .name
            .chars()
            .next()
            .map(|c| c.to_uppercase().to_string());

        self.db.get_conn().execute(
            "INSERT INTO accounts (id, name, auth_type, email, organization, is_default, is_active, created_at, updated_at, status, color, avatar_text)
             VALUES (?1, ?2, ?3, ?4, ?5, 0, 1, ?6, ?7, 'unknown', ?8, ?9)",
            params![id, input.name, auth_type.to_string(), input.email, input.organization, now, now, color, avatar_text],
        )?;

        // Encrypt and store credential
        let encrypted = security::encrypt(&input.credential_value)?;
        let cred_id = Uuid::new_v4().to_string();
        let cred_type = input
            .credential_type
            .unwrap_or_else(|| auth_type.to_string());

        self.db.get_conn().execute(
            "INSERT INTO credentials (id, account_id, credential_type, encrypted_value, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![cred_id, id, cred_type, encrypted, now, now],
        )?;

        self.get_by_id(&id)
    }

    pub fn get_by_id(&self, id: &str) -> AppResult<Account> {
        let conn = self.db.get_conn();
        let mut stmt = conn.prepare(
            "SELECT id, name, auth_type, email, organization, is_default, is_active, 
                    created_at, updated_at, last_checked_at, status, status_message, color, avatar_text
             FROM accounts WHERE id = ?1"
        )?;

        let account = stmt
            .query_row(params![id], |row| Self::map_row(row))
            .map_err(|_| AppError::AccountNotFound(id.to_string()))?;

        Ok(account)
    }

    pub fn list_all(&self) -> AppResult<Vec<Account>> {
        let conn = self.db.get_conn();
        let mut stmt = conn.prepare(
            "SELECT id, name, auth_type, email, organization, is_default, is_active,
                    created_at, updated_at, last_checked_at, status, status_message, color, avatar_text
             FROM accounts ORDER BY is_default DESC, created_at ASC"
        )?;

        let accounts = stmt
            .query_map([], |row| Self::map_row(row))?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(accounts)
    }

    pub fn update(&self, input: UpdateAccountInput) -> AppResult<Account> {
        let now = Utc::now().to_rfc3339();

        if let Some(name) = &input.name {
            let avatar_text = name.chars().next().map(|c| c.to_uppercase().to_string());
            self.db.get_conn().execute(
                "UPDATE accounts SET name = ?1, avatar_text = ?2, updated_at = ?3 WHERE id = ?4",
                params![name, avatar_text, now, input.id],
            )?;
        }

        if let Some(email) = &input.email {
            self.db.get_conn().execute(
                "UPDATE accounts SET email = ?1, updated_at = ?2 WHERE id = ?3",
                params![email, now, input.id],
            )?;
        }

        if let Some(org) = &input.organization {
            self.db.get_conn().execute(
                "UPDATE accounts SET organization = ?1, updated_at = ?2 WHERE id = ?3",
                params![org, now, input.id],
            )?;
        }

        if let Some(color) = &input.color {
            self.db.get_conn().execute(
                "UPDATE accounts SET color = ?1, updated_at = ?2 WHERE id = ?3",
                params![color, now, input.id],
            )?;
        }

        if let Some(cred) = &input.credential_value {
            let encrypted = security::encrypt(cred)?;
            self.db.get_conn().execute(
                "UPDATE credentials SET encrypted_value = ?1, updated_at = ?2 WHERE account_id = ?3",
                params![encrypted, now, input.id],
            )?;
        }

        self.get_by_id(&input.id)
    }

    pub fn upsert_synced_account(&self, input: UpsertSyncedAccountInput) -> AppResult<Account> {
        let UpsertSyncedAccountInput {
            stable_id,
            name,
            auth_type,
            email,
            organization,
            color,
            credential_value,
            credential_type,
        } = input;
        let now = Utc::now().to_rfc3339();
        let color = color.unwrap_or_else(|| "#4f8ef7".to_string());
        let avatar_text = name.chars().next().map(|c| c.to_uppercase().to_string());
        let account_exists = self.get_by_id(&stable_id).is_ok();
        let should_be_default = !account_exists && self.list_all()?.is_empty();

        self.db.get_conn().execute(
            "INSERT INTO accounts (id, name, auth_type, email, organization, is_default, is_active, created_at, updated_at, status, color, avatar_text)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, ?7, ?8, 'unknown', ?9, ?10)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                auth_type = excluded.auth_type,
                email = excluded.email,
                organization = excluded.organization,
                is_active = 1,
                updated_at = excluded.updated_at,
                color = excluded.color,
                avatar_text = excluded.avatar_text",
            params![
                &stable_id,
                &name,
                auth_type.to_string(),
                &email,
                &organization,
                if should_be_default { 1 } else { 0 },
                &now,
                &now,
                &color,
                &avatar_text,
            ],
        )?;

        let encrypted = security::encrypt(&credential_value)?;
        let credential_type = credential_type.unwrap_or_else(|| auth_type.to_string());
        let credential_id: Option<String> = self
            .db
            .get_conn()
            .query_row(
                "SELECT id FROM credentials WHERE account_id = ?1 LIMIT 1",
                params![&stable_id],
                |row| row.get(0),
            )
            .optional()?;

        if let Some(existing_credential_id) = credential_id {
            self.db.get_conn().execute(
                "UPDATE credentials
                 SET credential_type = ?1, encrypted_value = ?2, updated_at = ?3
                 WHERE id = ?4",
                params![credential_type, encrypted, &now, existing_credential_id],
            )?;
        } else {
            self.db.get_conn().execute(
                "INSERT INTO credentials (id, account_id, credential_type, encrypted_value, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    Uuid::new_v4().to_string(),
                    &stable_id,
                    credential_type,
                    encrypted,
                    &now,
                    &now,
                ],
            )?;
        }

        self.get_by_id(&stable_id)
    }

    pub fn delete(&self, id: &str) -> AppResult<()> {
        let affected = self
            .db
            .get_conn()
            .execute("DELETE FROM accounts WHERE id = ?1", params![id])?;

        if affected == 0 {
            return Err(AppError::AccountNotFound(id.to_string()));
        }

        Ok(())
    }

    pub fn set_default(&self, id: &str) -> AppResult<()> {
        let conn = self.db.get_conn();
        conn.execute("UPDATE accounts SET is_default = 0", [])?;
        conn.execute(
            "UPDATE accounts SET is_default = 1, updated_at = ?1 WHERE id = ?2",
            params![Utc::now().to_rfc3339(), id],
        )?;
        Ok(())
    }

    pub fn update_status(
        &self,
        id: &str,
        status: &AccountStatus,
        message: Option<&str>,
    ) -> AppResult<()> {
        let now = Utc::now().to_rfc3339();
        self.db.get_conn().execute(
            "UPDATE accounts SET status = ?1, status_message = ?2, last_checked_at = ?3, updated_at = ?4 WHERE id = ?5",
            params![status.to_string(), message, now, now, id],
        )?;
        Ok(())
    }

    pub fn get_credential(&self, account_id: &str) -> AppResult<String> {
        let conn = self.db.get_conn();
        let encrypted: String = conn
            .query_row(
                "SELECT encrypted_value FROM credentials WHERE account_id = ?1 LIMIT 1",
                params![account_id],
                |row| row.get(0),
            )
            .map_err(|_| AppError::AccountNotFound(account_id.to_string()))?;

        security::decrypt(&encrypted)
    }

    fn map_row(row: &Row) -> rusqlite::Result<Account> {
        let auth_type_str: String = row.get(2)?;
        let auth_type = AuthType::try_from(auth_type_str.as_str()).unwrap_or(AuthType::ApiKey);

        let status_str: String = row.get(10)?;
        let status = AccountStatus::from(status_str.as_str());

        Ok(Account {
            id: row.get(0)?,
            name: row.get(1)?,
            auth_type,
            email: row.get(3)?,
            organization: row.get(4)?,
            is_default: row.get::<_, i32>(5)? != 0,
            is_active: row.get::<_, i32>(6)? != 0,
            created_at: row.get(7)?,
            updated_at: row.get(8)?,
            last_checked_at: row.get(9)?,
            status,
            status_message: row.get(11)?,
            color: row.get(12)?,
            avatar_text: row.get(13)?,
        })
    }
}
