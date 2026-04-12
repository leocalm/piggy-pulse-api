use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Password reset record stored in the database
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PasswordReset {
    pub id: Uuid,
    pub user_id: Uuid,
    pub expires_at: DateTime<Utc>,
    pub used_at: Option<DateTime<Utc>>,
}

impl PasswordReset {
    /// Check if the token has expired
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    /// Check if the token has been used
    pub fn is_used(&self) -> bool {
        self.used_at.is_some()
    }

    /// Check if the token is still valid (not expired and not used)
    pub fn is_valid(&self) -> bool {
        !self.is_expired() && !self.is_used()
    }
}
