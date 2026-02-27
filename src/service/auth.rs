// src/service/auth.rs

use chrono::Utc;
use crate::Config;
use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::models::rate_limit::RateLimitStatus;
use uuid::Uuid;

/// What happened during a login attempt.
#[allow(dead_code)]
pub enum LoginOutcome {
    /// Credentials valid and session created successfully.
    Success { session_id: Uuid, user_id: Uuid },
    /// Credentials valid but 2FA code is required before a session is issued.
    TwoFactorRequired,
}

#[allow(dead_code)]
pub struct AuthService<'a> {
    pub repo: &'a PostgresRepository,
    pub config: &'a Config,
}

impl<'a> AuthService<'a> {
    /// Checks the pre-login rate limit for a user (by id) and IP.
    /// Returns `Ok(())` if the request is allowed, or an `Err(AppError)` if
    /// it should be rejected (Delayed or Locked).
    pub async fn check_login_rate_limit(
        &self,
        user_id: Option<&Uuid>,
        ip: &str,
        user_email: Option<&str>,
        user_name: Option<&str>,
    ) -> Result<(), AppError> {
        let status = self.repo.check_login_rate_limit(user_id, ip).await?;
        match status {
            RateLimitStatus::Delayed { until } => {
                let seconds_remaining = (until - Utc::now()).num_seconds().max(0);
                Err(AppError::TooManyAttempts {
                    retry_after_seconds: seconds_remaining,
                    message: "Too many failed attempts. Please wait before trying again."
                        .to_string(),
                })
            }
            RateLimitStatus::Locked { until, can_unlock } => {
                if can_unlock
                    && self.config.login_rate_limit.enable_email_unlock
                    && let Some(uid) = user_id
                    && let Some(email) = user_email
                    && let Some(name) = user_name
                    && let Ok(token) = self.repo.create_unlock_token(uid).await
                {
                    let email_service =
                        crate::service::email::EmailService::new(self.config.email.clone());
                    let _ = email_service
                        .send_account_locked_email(
                            email,
                            name,
                            &uid.to_string(),
                            &token,
                            &self.config.login_rate_limit.frontend_unlock_url,
                        )
                        .await;
                }
                Err(AppError::AccountLocked {
                    locked_until: until,
                    message: "Account temporarily locked due to too many failed attempts. Check your email for unlock instructions.".to_string(),
                })
            }
            RateLimitStatus::Allowed => Ok(()),
        }
    }
}
