use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Database model for two-factor authentication
#[derive(sqlx::FromRow)]
pub struct TwoFactorAuth {
    pub encrypted_secret: String,
    pub encryption_nonce: String,
    pub is_enabled: bool,
}

/// Backup code database model
#[derive(Debug, sqlx::FromRow)]
pub struct BackupCode {
    pub id: Uuid,
    pub code_hash: String,
}

/// Rate limit database model
#[derive(Debug, sqlx::FromRow)]
pub struct TwoFactorRateLimit {
    pub failed_attempts: i32,
    pub locked_until: Option<DateTime<Utc>>,
}

/// Emergency disable token model
#[derive(Debug, sqlx::FromRow)]
pub struct EmergencyToken {
    pub id: Uuid,
    pub user_id: Uuid,
}
