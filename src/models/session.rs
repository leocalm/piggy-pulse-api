use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, sqlx::FromRow)]
pub struct Session {
    pub id: Uuid,
}

#[derive(Debug, sqlx::FromRow)]
pub struct SessionUser {
    pub id: Uuid,
    pub email: String,
}

/// Full session row including device metadata captured at login.
#[derive(Debug, sqlx::FromRow)]
pub struct SessionInfoFull {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub user_agent: Option<String>,
}
