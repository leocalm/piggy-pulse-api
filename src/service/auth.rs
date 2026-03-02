// src/service/auth.rs

use crate::Config;
use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::models::audit::audit_events;
use crate::models::rate_limit::RateLimitStatus;
use chrono::Utc;
use uuid::Uuid;

/// What happened during a login attempt.
pub enum LoginOutcome {
    /// Credentials valid and session created successfully.
    Success { session_id: Uuid, user_id: Uuid },
    /// Credentials valid but 2FA code is required before a session is issued.
    TwoFactorRequired,
}

pub struct AuthService<'a> {
    pub repo: &'a PostgresRepository,
    pub config: &'a Config,
}

impl<'a> AuthService<'a> {
    pub fn new(repo: &'a PostgresRepository, config: &'a Config) -> Self {
        AuthService { repo, config }
    }

    /// Full login flow. Returns LoginOutcome on success, or AppError on failure.
    pub async fn login(
        &self,
        payload: &crate::models::user::LoginRequest,
        ip: &str,
        client_ip: Option<String>,
        user_agent: Option<String>,
    ) -> Result<LoginOutcome, AppError> {
        let user_opt = self.repo.get_user_by_email(&payload.email).await?;
        let user_id = user_opt.as_ref().map(|u| u.id);

        // Pre-login rate limit check
        self.check_login_rate_limit(user_id.as_ref(), ip, user_opt.as_ref().map(|u| (u.email.as_str(), u.name.as_str())))
            .await?;

        let user = match user_opt {
            Some(u) => u,
            None => {
                PostgresRepository::dummy_verify(&payload.password);
                let _ = self.repo.record_failed_login_attempt(None, ip, &self.config.login_rate_limit).await;
                let _ = self
                    .repo
                    .create_security_audit_log(
                        None,
                        audit_events::LOGIN_FAILED,
                        false,
                        client_ip,
                        user_agent,
                        Some(serde_json::json!({"reason": "user_not_found"})),
                    )
                    .await;
                return Err(AppError::InvalidCredentials);
            }
        };

        // Password verification
        if self.repo.verify_password(&user, &payload.password).await.is_err() {
            return Err(self.handle_failed_password(&user.id, &user.email, &user.name, ip, client_ip, user_agent).await);
        }

        // Reset login rate limits on success
        let _ = self.repo.reset_login_rate_limit(&user.id, ip).await;

        // 2FA check
        let two_factor = self.repo.get_two_factor_by_user(&user.id).await?;
        let has_2fa = two_factor.as_ref().map(|tf| tf.is_enabled).unwrap_or(false);

        if has_2fa {
            let code = match payload.two_factor_code.as_ref() {
                Some(c) => c,
                None => return Ok(LoginOutcome::TwoFactorRequired),
            };

            let two_factor_data = two_factor.unwrap();
            let backup_code_used = self
                .verify_two_factor(&user.id, two_factor_data, code, client_ip.clone(), user_agent.clone())
                .await?;

            if backup_code_used {
                let _ = self
                    .repo
                    .create_security_audit_log(
                        Some(&user.id),
                        audit_events::TWO_FACTOR_BACKUP_USED,
                        true,
                        client_ip.clone(),
                        user_agent.clone(),
                        None,
                    )
                    .await;
            }
        }

        // Create session
        let ttl_seconds = self.config.session.ttl_seconds.max(60);
        let expires_at = chrono::Utc::now() + chrono::Duration::seconds(ttl_seconds);
        let session = self
            .repo
            .create_session(&user.id, expires_at, user_agent.as_deref(), client_ip.as_deref())
            .await?;

        let _ = self
            .repo
            .create_security_audit_log(
                Some(&user.id),
                audit_events::LOGIN_SUCCESS,
                true,
                client_ip,
                user_agent,
                Some(serde_json::json!({
                    "email": &payload.email,
                    "2fa_used": has_2fa,
                })),
            )
            .await;

        Ok(LoginOutcome::Success {
            session_id: session.id,
            user_id: user.id,
        })
    }

    /// Checks the pre-login rate limit for a user (by id) and IP.
    /// Returns `Ok(())` if the request is allowed, or an `Err(AppError)` if
    /// it should be rejected (Delayed or Locked).
    ///
    /// `user_contact` is `Some((email, name))` when the user record was resolved;
    /// `None` when the user was not found. The unlock email is only sent when a
    /// user record is available, preventing email sends for IP-only locks.
    pub async fn check_login_rate_limit(&self, user_id: Option<&Uuid>, ip: &str, user_contact: Option<(&str, &str)>) -> Result<(), AppError> {
        let status = self.repo.check_login_rate_limit(user_id, ip).await?;
        match status {
            RateLimitStatus::Delayed { until } => {
                let seconds_remaining = (until - Utc::now()).num_seconds().max(0);
                Err(AppError::TooManyAttempts {
                    retry_after_seconds: seconds_remaining,
                    message: "Too many failed attempts. Please wait before trying again.".to_string(),
                })
            }
            RateLimitStatus::Locked { until, can_unlock } => {
                if can_unlock
                    && self.config.login_rate_limit.enable_email_unlock
                    && let Some(uid) = user_id
                    && let Some((email, name)) = user_contact
                    && let Ok(token) = self.repo.create_unlock_token(uid).await
                {
                    let email_service = crate::service::email::EmailService::new(self.config.email.clone());
                    let _ = email_service
                        .send_account_locked_email(email, name, &uid.to_string(), &token, &self.config.login_rate_limit.frontend_unlock_url)
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

    /// Records a failed password attempt, writes an audit log, and returns the
    /// appropriate error (rate-limited variants take priority over plain 401).
    pub async fn handle_failed_password(
        &self,
        user_id: &Uuid,
        user_email: &str,
        user_name: &str,
        ip: &str,
        client_ip: Option<String>,
        user_agent: Option<String>,
    ) -> AppError {
        let new_status = self.repo.record_failed_login_attempt(Some(user_id), ip, &self.config.login_rate_limit).await;

        let _ = self
            .repo
            .create_security_audit_log(
                Some(user_id),
                audit_events::LOGIN_FAILED,
                false,
                client_ip.clone(),
                user_agent.clone(),
                Some(serde_json::json!({"reason": "invalid_password"})),
            )
            .await;

        match new_status {
            Ok(RateLimitStatus::Delayed { until }) => {
                let seconds_remaining = (until - Utc::now()).num_seconds().max(0);
                AppError::TooManyAttempts {
                    retry_after_seconds: seconds_remaining,
                    message: "Too many failed attempts. Please wait before trying again.".to_string(),
                }
            }
            Ok(RateLimitStatus::Locked { until, can_unlock }) => {
                if can_unlock && self.config.login_rate_limit.enable_email_unlock {
                    if let Ok(token) = self.repo.create_unlock_token(user_id).await {
                        let email_service = crate::service::email::EmailService::new(self.config.email.clone());
                        let _ = email_service
                            .send_account_locked_email(
                                user_email,
                                user_name,
                                &user_id.to_string(),
                                &token,
                                &self.config.login_rate_limit.frontend_unlock_url,
                            )
                            .await;
                    } else {
                        tracing::warn!("Failed to create unlock token for user {}", user_id);
                    }
                }
                AppError::AccountLocked {
                    locked_until: until,
                    message: "Account temporarily locked due to too many failed attempts. Check your email for unlock instructions.".to_string(),
                }
            }
            _ => AppError::InvalidCredentials,
        }
    }

    /// Verifies a 2FA code (TOTP or backup) for the given user.
    /// Returns `Ok(backup_used)` on success, or an `Err(AppError)` on failure.
    pub async fn verify_two_factor(
        &self,
        user_id: &Uuid,
        two_factor_data: crate::models::two_factor::TwoFactorAuth,
        code: &str,
        client_ip: Option<String>,
        user_agent: Option<String>,
    ) -> Result<bool, AppError> {
        // Check 2FA-specific rate limit
        if self.repo.check_rate_limit(user_id).await? {
            return Err(AppError::BadRequest("Too many failed attempts. Please try again later.".to_string()));
        }

        let encryption_key = self.config.two_factor.parse_encryption_key().map_err(AppError::BadRequest)?;

        let encrypted_secret = two_factor_data.encrypted_secret.clone();
        let encryption_nonce = two_factor_data.encryption_nonce.clone();
        let code_owned = code.to_string();

        let totp_valid = tokio::task::spawn_blocking(move || {
            let secret = PostgresRepository::decrypt_secret(&encrypted_secret, &encryption_nonce, &encryption_key)?;
            PostgresRepository::verify_totp_code(&secret, &code_owned)
        })
        .await
        .map_err(|e| AppError::BadRequest(format!("Task join error: {}", e)))??;

        let backup_valid = if !totp_valid {
            self.repo.verify_backup_code(user_id, code).await?
        } else {
            false
        };

        if !totp_valid && !backup_valid {
            self.repo.record_failed_attempt(user_id).await?;
            let _ = self
                .repo
                .create_security_audit_log(
                    Some(user_id),
                    audit_events::LOGIN_FAILED,
                    false,
                    client_ip,
                    user_agent,
                    Some(serde_json::json!({"reason": "invalid_2fa_code"})),
                )
                .await;
            return Err(AppError::BadRequest("Invalid two-factor authentication code.".to_string()));
        }

        self.repo.reset_rate_limit(user_id).await?;
        Ok(backup_valid)
    }
}
