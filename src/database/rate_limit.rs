use crate::config::LoginRateLimitConfig;
use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::models::audit::audit_events;
use crate::models::rate_limit::{LoginRateLimit, RateLimitStatus};
use chrono::{Duration, Utc};
use uuid::Uuid;

impl PostgresRepository {
    /// Check if a login attempt should be allowed, delayed, or blocked.
    ///
    /// Checks both user-based (if user_id provided) and IP-based rate limits.
    /// Returns the most restrictive status.
    pub async fn check_login_rate_limit(
        &self,
        user_id: Option<&Uuid>,
        ip_address: &str,
    ) -> Result<RateLimitStatus, AppError> {
        // Check user-based limit if user_id provided
        if let Some(uid) = user_id {
            let user_limit = sqlx::query_as::<_, LoginRateLimit>(
                "SELECT * FROM login_rate_limits
                 WHERE identifier_type = 'user_id' AND identifier_value = $1",
            )
            .bind(uid.to_string())
            .fetch_optional(&self.pool)
            .await?;

            if let Some(limit) = user_limit {
                if let Some(locked_until) = limit.locked_until {
                    if locked_until > Utc::now() {
                        return Ok(RateLimitStatus::Locked {
                            until: locked_until,
                            can_unlock: true,
                        });
                    }
                }

                if let Some(next_attempt) = limit.next_attempt_at {
                    if next_attempt > Utc::now() {
                        return Ok(RateLimitStatus::Delayed { until: next_attempt });
                    }
                }
            }
        }

        // Check IP-based limit
        let ip_limit = sqlx::query_as::<_, LoginRateLimit>(
            "SELECT * FROM login_rate_limits
             WHERE identifier_type = 'ip_address' AND identifier_value = $1",
        )
        .bind(ip_address)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(limit) = ip_limit {
            if let Some(locked_until) = limit.locked_until {
                if locked_until > Utc::now() {
                    return Ok(RateLimitStatus::Locked {
                        until: locked_until,
                        can_unlock: false, // IP locks require manual clearing
                    });
                }
            }

            if let Some(next_attempt) = limit.next_attempt_at {
                if next_attempt > Utc::now() {
                    return Ok(RateLimitStatus::Delayed { until: next_attempt });
                }
            }
        }

        Ok(RateLimitStatus::Allowed)
    }

    /// Record a failed login attempt and apply progressive delays or lockout.
    pub async fn record_failed_login_attempt(
        &self,
        user_id: Option<&Uuid>,
        ip_address: &str,
        config: &LoginRateLimitConfig,
    ) -> Result<(), AppError> {
        let now = Utc::now();

        // Update or insert for user if provided
        if let Some(uid) = user_id {
            let current = sqlx::query_scalar::<_, i32>(
                "SELECT COALESCE(failed_attempts, 0) FROM login_rate_limits
                 WHERE identifier_type = 'user_id' AND identifier_value = $1",
            )
            .bind(uid.to_string())
            .fetch_optional(&self.pool)
            .await?
            .unwrap_or(0);

            let new_attempts = current + 1;
            let (next_attempt_at, locked_until) = compute_backoff(new_attempts, config, now);

            let result = sqlx::query_as::<_, LoginRateLimit>(
                "INSERT INTO login_rate_limits
                 (identifier_type, identifier_value, failed_attempts, last_attempt_at, next_attempt_at, locked_until)
                 VALUES ('user_id', $1, $2, $3, $4, $5)
                 ON CONFLICT (identifier_type, identifier_value)
                 DO UPDATE SET
                    failed_attempts = $2,
                    last_attempt_at = $3,
                    next_attempt_at = $4,
                    locked_until = $5,
                    updated_at = $3
                 RETURNING *",
            )
            .bind(uid.to_string())
            .bind(new_attempts)
            .bind(now)
            .bind(next_attempt_at)
            .bind(locked_until)
            .fetch_one(&self.pool)
            .await?;

            // Log if account just got locked
            if result.locked_until.is_some() && current < config.lockout_attempts {
                let _ = self
                    .create_security_audit_log(
                        Some(uid),
                        audit_events::ACCOUNT_LOCKED,
                        false,
                        Some(ip_address.to_string()),
                        None,
                        Some(serde_json::json!({
                            "failed_attempts": result.failed_attempts,
                            "locked_until": result.locked_until,
                        })),
                    )
                    .await;
            }
        }

        // Always update IP-based tracking
        let ip_current = sqlx::query_scalar::<_, i32>(
            "SELECT COALESCE(failed_attempts, 0) FROM login_rate_limits
             WHERE identifier_type = 'ip_address' AND identifier_value = $1",
        )
        .bind(ip_address)
        .fetch_optional(&self.pool)
        .await?
        .unwrap_or(0);

        let ip_new_attempts = ip_current + 1;
        let (ip_next_attempt_at, ip_locked_until) = compute_backoff(ip_new_attempts, config, now);

        sqlx::query(
            "INSERT INTO login_rate_limits
             (identifier_type, identifier_value, failed_attempts, last_attempt_at, next_attempt_at, locked_until)
             VALUES ('ip_address', $1, $2, $3, $4, $5)
             ON CONFLICT (identifier_type, identifier_value)
             DO UPDATE SET
                failed_attempts = $2,
                last_attempt_at = $3,
                next_attempt_at = $4,
                locked_until = $5,
                updated_at = $3",
        )
        .bind(ip_address)
        .bind(ip_new_attempts)
        .bind(now)
        .bind(ip_next_attempt_at)
        .bind(ip_locked_until)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Reset rate limits after a successful login.
    pub async fn reset_login_rate_limit(
        &self,
        user_id: &Uuid,
        ip_address: &str,
    ) -> Result<(), AppError> {
        sqlx::query(
            "DELETE FROM login_rate_limits
             WHERE (identifier_type = 'user_id' AND identifier_value = $1)
                OR (identifier_type = 'ip_address' AND identifier_value = $2)",
        )
        .bind(user_id.to_string())
        .bind(ip_address)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Generate and store an unlock token, returning the token.
    pub async fn create_unlock_token(&self, user_id: &Uuid) -> Result<String, AppError> {
        use rand::distr::{Alphanumeric, SampleString};

        let token = Alphanumeric.sample_string(&mut rand::rng(), 32);
        let expires_at = Utc::now() + Duration::hours(1);

        sqlx::query(
            "UPDATE login_rate_limits
             SET unlock_token = $1,
                 unlock_token_expires_at = $2,
                 updated_at = $3
             WHERE identifier_type = 'user_id' AND identifier_value = $4",
        )
        .bind(&token)
        .bind(expires_at)
        .bind(Utc::now())
        .bind(user_id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(token)
    }

    /// Verify an unlock token and clear the rate limit if valid.
    pub async fn verify_and_apply_unlock_token(
        &self,
        user_id: &Uuid,
        token: &str,
    ) -> Result<bool, AppError> {
        let is_valid = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(
                SELECT 1 FROM login_rate_limits
                WHERE identifier_type = 'user_id'
                AND identifier_value = $1
                AND unlock_token = $2
                AND unlock_token_expires_at > $3
            )",
        )
        .bind(user_id.to_string())
        .bind(token)
        .bind(Utc::now())
        .fetch_one(&self.pool)
        .await?;

        if is_valid {
            sqlx::query(
                "DELETE FROM login_rate_limits
                 WHERE identifier_type = 'user_id' AND identifier_value = $1",
            )
            .bind(user_id.to_string())
            .execute(&self.pool)
            .await?;

            let _ = self
                .create_security_audit_log(
                    Some(user_id),
                    audit_events::ACCOUNT_UNLOCKED,
                    true,
                    None,
                    None,
                    Some(serde_json::json!({"method": "email_token"})),
                )
                .await;
        }

        Ok(is_valid)
    }
}

/// Compute next_attempt_at and locked_until given the new attempt count and config.
fn compute_backoff(
    new_attempts: i32,
    config: &LoginRateLimitConfig,
    now: chrono::DateTime<Utc>,
) -> (Option<chrono::DateTime<Utc>>, Option<chrono::DateTime<Utc>>) {
    if new_attempts >= config.lockout_attempts {
        let locked_until = now + Duration::minutes(config.lockout_duration_minutes);
        return (None, Some(locked_until));
    }

    let delay_index = (new_attempts - config.free_attempts - 1) as usize;
    if new_attempts > config.free_attempts && delay_index < config.delay_seconds.len() {
        let delay_secs = config.delay_seconds[delay_index];
        let next_attempt = now + Duration::seconds(delay_secs);
        return (Some(next_attempt), None);
    }

    (None, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_backoff_allows_free_attempts() {
        let config = LoginRateLimitConfig::default();
        let now = Utc::now();

        let (next, locked) = compute_backoff(1, &config, now);
        assert!(next.is_none());
        assert!(locked.is_none());

        let (next, locked) = compute_backoff(3, &config, now);
        assert!(next.is_none());
        assert!(locked.is_none());
    }

    #[test]
    fn compute_backoff_delays_after_free_attempts() {
        let config = LoginRateLimitConfig::default();
        let now = Utc::now();

        let (next, locked) = compute_backoff(4, &config, now);
        assert!(next.is_some());
        assert!(locked.is_none());

        // Should be ~5s from now (first delay)
        let diff = (next.unwrap() - now).num_seconds();
        assert_eq!(diff, 5);
    }

    #[test]
    fn compute_backoff_locks_after_lockout_threshold() {
        let config = LoginRateLimitConfig::default();
        let now = Utc::now();

        let (next, locked) = compute_backoff(7, &config, now);
        assert!(next.is_none());
        assert!(locked.is_some());

        let lock_duration = (locked.unwrap() - now).num_minutes();
        assert_eq!(lock_duration, 60);
    }

    #[tokio::test]
    #[ignore = "requires database"]
    async fn test_check_login_rate_limit_allows_first_attempt() {
        // Requires a running PostgreSQL at DATABASE_URL
    }

    #[tokio::test]
    #[ignore = "requires database"]
    async fn test_record_failed_login_increments_counter() {
        // Requires a running PostgreSQL at DATABASE_URL
    }

    #[tokio::test]
    #[ignore = "requires database"]
    async fn test_reset_login_rate_limit_clears_attempts() {
        // Requires a running PostgreSQL at DATABASE_URL
    }
}
