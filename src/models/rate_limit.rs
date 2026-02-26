#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum RateLimitStatus {
    Allowed,
    Delayed { until: DateTime<Utc> },
    Locked { until: DateTime<Utc>, can_unlock: bool },
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct LoginRateLimit {
    pub id: Uuid,
    pub identifier_type: String,
    pub identifier_value: String,
    pub failed_attempts: i32,
    pub last_attempt_at: DateTime<Utc>,
    pub locked_until: Option<DateTime<Utc>>,
    pub next_attempt_at: Option<DateTime<Utc>>,
    pub unlock_token: Option<String>,
    pub unlock_token_expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limit_status_variants() {
        let allowed = RateLimitStatus::Allowed;
        assert!(matches!(allowed, RateLimitStatus::Allowed));

        let delayed = RateLimitStatus::Delayed { until: Utc::now() };
        assert!(matches!(delayed, RateLimitStatus::Delayed { .. }));

        let locked = RateLimitStatus::Locked {
            until: Utc::now(),
            can_unlock: true,
        };
        assert!(matches!(locked, RateLimitStatus::Locked { .. }));
    }
}
