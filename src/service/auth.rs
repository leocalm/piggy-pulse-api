// src/service/auth.rs

use crate::Config;
use crate::database::postgres_repository::{PostgresRepository, is_unique_violation};
use crate::error::app_error::AppError;
use crate::models::audit::audit_events;
use crate::models::rate_limit::RateLimitStatus;
use crate::models::user::User;
use chrono::Utc;
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// What happened during a V2 login attempt.
pub enum V2LoginOutcome {
    /// Credentials valid and session created successfully.
    Success { session_id: Uuid, user: User },
    /// Credentials valid but 2FA is required; a pending token was issued.
    TwoFactorRequired { two_factor_token: String },
}

pub struct AuthService<'a> {
    pub repo: &'a PostgresRepository,
    pub config: &'a Config,
}

impl<'a> AuthService<'a> {
    pub fn new(repo: &'a PostgresRepository, config: &'a Config) -> Self {
        AuthService { repo, config }
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

    // ─── V2 methods ─────────────────────────────────────────────────────────

    /// Register a new user, create default resources, and start a session.
    pub async fn register(&self, email: &str, password: &str, name: &str, user_agent: Option<&str>, client_ip: Option<&str>) -> Result<(User, Uuid), AppError> {
        let user = self.repo.create_user(name, email, password).await.map_err(|e| {
            if let AppError::Db { ref source, .. } = e
                && is_unique_violation(source)
            {
                return AppError::UserAlreadyExists(email.to_string());
            }
            e
        })?;

        // Best-effort: create default settings (no currency chosen yet — done in onboarding)
        if let Err(e) = self.repo.create_default_settings(&user.id).await {
            tracing::warn!("Failed to create default settings for user {}: {}", user.id, e);
        }

        // Best-effort: create system transfer category
        if let Err(e) = self.repo.create_system_transfer_category(&user.id).await {
            tracing::warn!("Failed to create system transfer category for user {}: {}", user.id, e);
        }

        // Best-effort: send welcome email
        let email_service = crate::service::email::EmailService::new(self.config.email.clone());
        if let Err(e) = email_service.send_welcome_email(email, name).await {
            tracing::warn!("Failed to send welcome email to {}: {}", email, e);
        }

        // Create session
        let ttl_seconds = self.config.session.ttl_seconds.max(60);
        let expires_at = Utc::now() + chrono::Duration::seconds(ttl_seconds);
        let session = self.repo.create_session(&user.id, expires_at, user_agent, client_ip).await?;

        Ok((user, session.id))
    }

    /// V2 login flow. Returns V2LoginOutcome on success.
    /// On 2FA required: creates a pending 2FA token and returns it.
    pub async fn login_v2(
        &self,
        email: &str,
        password: &str,
        ip: &str,
        client_ip: Option<String>,
        user_agent: Option<String>,
    ) -> Result<V2LoginOutcome, AppError> {
        let user_opt = self.repo.get_user_by_email(email).await?;
        let user_id = user_opt.as_ref().map(|u| u.id);

        // Pre-login rate limit check
        self.check_login_rate_limit(user_id.as_ref(), ip, user_opt.as_ref().map(|u| (u.email.as_str(), u.name.as_str())))
            .await?;

        let user = match user_opt {
            Some(u) => u,
            None => {
                PostgresRepository::dummy_verify(password);
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
        if self.repo.verify_password(&user, password).await.is_err() {
            return Err(self.handle_failed_password(&user.id, &user.email, &user.name, ip, client_ip, user_agent).await);
        }

        // Reset login rate limits on success
        let _ = self.repo.reset_login_rate_limit(&user.id, ip).await;

        // 2FA check
        let two_factor = self.repo.get_two_factor_by_user(&user.id).await?;
        let has_2fa = two_factor.as_ref().map(|tf| tf.is_enabled).unwrap_or(false);

        if has_2fa {
            // Issue a short-lived pending token for the 2FA step
            let (two_fa_plain, two_fa_hash) = crate::models::api_token::generate_token("pp_2fa_");
            let pending_expires_at = Utc::now() + chrono::Duration::seconds(300);
            self.repo
                .create_pending_2fa_token(
                    &user.id,
                    &two_fa_hash,
                    user_agent.as_deref().unwrap_or("web"),
                    client_ip.as_deref().unwrap_or("web"),
                    &pending_expires_at,
                )
                .await?;

            return Ok(V2LoginOutcome::TwoFactorRequired {
                two_factor_token: two_fa_plain,
            });
        }

        // Create session
        let ttl_seconds = self.config.session.ttl_seconds.max(60);
        let expires_at = Utc::now() + chrono::Duration::seconds(ttl_seconds);
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
                    "email": email,
                    "2fa_used": false,
                })),
            )
            .await;

        Ok(V2LoginOutcome::Success { session_id: session.id, user })
    }

    /// Build a UserResponse DTO for V2 endpoints.
    pub async fn get_user_response(&self, user_id: &Uuid) -> Result<crate::dto::auth::UserResponse, AppError> {
        let user = self.repo.get_user_by_id(user_id).await?.ok_or(AppError::UserNotFound)?;
        self.build_user_response(user).await
    }

    /// Build a UserResponse DTO from an already-loaded User (avoids redundant DB fetch).
    pub async fn build_user_response(&self, user: User) -> Result<crate::dto::auth::UserResponse, AppError> {
        // Resolve currency code from settings
        let settings = self.repo.get_settings(&user.id).await?;
        let currency_code = if let Some(currency_id) = settings.default_currency_id {
            self.repo
                .get_currency_by_id(&currency_id)
                .await?
                .map(|c| c.currency)
                .unwrap_or_else(|| "USD".to_string())
        } else {
            "USD".to_string()
        };

        // 2FA status
        let two_factor_enabled = self.repo.get_two_factor_by_user(&user.id).await?.map(|tf| tf.is_enabled).unwrap_or(false);

        Ok(crate::dto::auth::UserResponse {
            id: user.id,
            email: user.email,
            name: user.name,
            currency: currency_code,
            two_factor_enabled,
        })
    }

    /// Change password for V2 (maps wrong-current-password to 401).
    /// Invalidates all other sessions after successful change.
    pub async fn change_password(
        &self,
        user_id: &Uuid,
        current_session_id: Option<Uuid>,
        current_password: &str,
        new_password: &str,
        client_ip: Option<String>,
        user_agent: Option<String>,
    ) -> Result<(), AppError> {
        // Manually verify current password then update, so we can map the error to 401
        let user = self.repo.get_user_by_id(user_id).await?.ok_or(AppError::UserNotFound)?;
        self.repo
            .verify_password(&user, current_password)
            .await
            .map_err(|_| AppError::InvalidCredentials)?;

        self.repo.update_user_password(user_id, new_password).await?;

        // Invalidate all other sessions (keep the current one)
        if let Some(session_id) = current_session_id {
            let _ = self.repo.delete_other_sessions_for_user(user_id, &session_id).await;
        } else {
            let _ = self.repo.delete_all_sessions_for_user(user_id).await;
        }

        // Revoke all bearer tokens — force re-authentication
        let _ = self.repo.revoke_all_for_user(user_id).await;

        let _ = self
            .repo
            .create_security_audit_log(Some(user_id), audit_events::PASSWORD_CHANGED, true, client_ip, user_agent, None)
            .await;

        // Best-effort: send password-changed security notification
        let changed_at = Utc::now().format("%b %d, %Y at %-I:%M %p").to_string();
        let email_service = crate::service::email::EmailService::new(self.config.email.clone());
        if let Err(e) = email_service.send_password_changed_email(&user.email, &user.name, &changed_at).await {
            tracing::warn!("Failed to send password changed email to {}: {}", user.email, e);
        }

        Ok(())
    }

    /// Request a password reset email. Always returns Ok for anti-enumeration.
    pub async fn forgot_password(&self, email: &str) -> Result<(), AppError> {
        let user = match self.repo.get_user_by_email(email).await? {
            Some(u) => u,
            None => {
                PostgresRepository::dummy_verify_no_input();
                return Ok(());
            }
        };

        // Rate limit check
        let since = Utc::now() - chrono::Duration::hours(1);
        let attempts = self.repo.count_password_reset_attempts(&user.id, since).await?;
        if attempts >= self.config.password_reset.max_attempts_per_hour as i64 {
            return Ok(()); // silently bail to prevent enumeration
        }

        // Generate and store reset token
        let (plain_token, token_hash) = PostgresRepository::generate_reset_token();
        let expires_at = Utc::now() + chrono::Duration::seconds(self.config.password_reset.token_ttl_seconds);
        self.repo.create_password_reset(&user.id, &token_hash, expires_at, None, None).await?;

        let _ = self
            .repo
            .create_security_audit_log(
                Some(&user.id),
                audit_events::PASSWORD_RESET_REQUESTED,
                true,
                None,
                None,
                Some(serde_json::json!({"email": email})),
            )
            .await;

        // Send email (best-effort)
        let email_service = crate::service::email::EmailService::new(self.config.email.clone());
        if let Err(e) = email_service
            .send_password_reset_email(&user.email, &user.name, &plain_token, &self.config.password_reset.frontend_reset_url)
            .await
        {
            tracing::error!("Failed to send password reset email: {}", e);
        }

        Ok(())
    }

    /// Reset a password using a token. Returns Unauthorized on invalid/expired token.
    pub async fn reset_password(&self, token: &str, new_password: &str) -> Result<(), AppError> {
        let token_hash = hex::encode(Sha256::digest(token.as_bytes()));

        let reset = self.repo.get_password_reset_by_token(&token_hash).await?.ok_or(AppError::Unauthorized)?;

        if !reset.is_valid() {
            return Err(AppError::Unauthorized);
        }

        // Update password
        self.repo.update_user_password(&reset.user_id, new_password).await?;

        // Mark token as used
        self.repo.mark_password_reset_used(&reset.id).await?;

        // Invalidate all sessions and revoke all bearer tokens
        let sessions_invalidated = self.repo.invalidate_all_user_sessions(&reset.user_id).await?;
        let _ = self.repo.revoke_all_for_user(&reset.user_id).await;

        // Clean up remaining reset tokens
        self.repo.delete_password_resets_for_user(&reset.user_id).await?;

        let _ = self
            .repo
            .create_security_audit_log(
                Some(&reset.user_id),
                audit_events::PASSWORD_RESET_COMPLETED,
                true,
                None,
                None,
                Some(serde_json::json!({"sessions_invalidated": sessions_invalidated})),
            )
            .await;

        // Best-effort: send password-changed security notification
        if let Ok(Some(user)) = self.repo.get_user_by_id(&reset.user_id).await {
            let changed_at = Utc::now().format("%b %d, %Y at %-I:%M %p").to_string();
            let email_service = crate::service::email::EmailService::new(self.config.email.clone());
            if let Err(e) = email_service.send_password_changed_email(&user.email, &user.name, &changed_at).await {
                tracing::warn!("Failed to send password changed email to {}: {}", user.email, e);
            }
        }

        Ok(())
    }

    /// Refresh a cookie-based session by replacing the current session with a new one.
    /// Returns the new session_id so the caller can re-stamp the cookie.
    pub async fn refresh_session(&self, user_id: &Uuid, session_id: Option<Uuid>, user_agent: Option<&str>, client_ip: Option<&str>) -> Result<Uuid, AppError> {
        // Delete the old session
        if let Some(sid) = session_id {
            let _ = self.repo.delete_session(&sid).await;
        }

        // Create a fresh session with full TTL
        let ttl_seconds = self.config.session.ttl_seconds.max(60);
        let expires_at = Utc::now() + chrono::Duration::seconds(ttl_seconds);
        let new_session = self.repo.create_session(user_id, expires_at, user_agent, client_ip).await?;

        Ok(new_session.id)
    }

    /// Issue a bearer access token for a user.
    /// Returns `(access_token_plaintext, access_ttl_seconds)`.
    pub async fn issue_bearer_token(&self, user_id: &Uuid) -> Result<(String, i64), AppError> {
        use crate::models::api_token::generate_token;

        let access_secs = self.config.session.access_token_ttl_seconds.unwrap_or(3600);
        // V2 does not use refresh tokens, but refresh_expires_at controls when
        // cleanup_expired() deletes the row — effectively the max token lifetime.
        let refresh_secs = self.config.session.refresh_token_ttl_seconds.unwrap_or(30 * 24 * 3600);

        let (access_plain, access_hash) = generate_token("pp_at_");
        // V2 uses access-token-based refresh (via token row ID), not refresh tokens.
        // Store a sentinel hash so the column is non-empty but never matches a lookup.
        let refresh_hash = "v2_no_refresh".to_string();

        let now = Utc::now();
        let expires_at = now + chrono::Duration::seconds(access_secs);
        let refresh_expires_at = now + chrono::Duration::seconds(refresh_secs);

        // Use a stable device_id so the ON CONFLICT upsert replaces the previous
        // v2 token for this user, preventing unbounded row accumulation.
        let device_id = format!("v2_{}", user_id);

        self.repo
            .create_api_token(
                user_id,
                access_hash,
                refresh_hash,
                "v2".to_string(),
                &device_id,
                &expires_at,
                &refresh_expires_at,
            )
            .await?;

        Ok((access_plain, access_secs))
    }

    /// Rotate the bearer access token for an existing API token row identified by its DB row ID.
    /// Returns the new plaintext access token.
    pub async fn refresh_bearer_token_by_id(&self, token_id: &Uuid) -> Result<String, AppError> {
        use crate::models::api_token::generate_token;

        let access_secs = self.config.session.access_token_ttl_seconds.unwrap_or(3600);
        let (access_plain, access_hash) = generate_token("pp_at_");
        let new_expires_at = Utc::now() + chrono::Duration::seconds(access_secs);

        self.repo.update_access_token(token_id, access_hash, &new_expires_at).await?;

        Ok(access_plain)
    }

    /// Log out by deleting the session and recording an audit event.
    pub async fn logout(&self, user_id: &Uuid, session_id: Option<Uuid>, client_ip: Option<String>, user_agent: Option<String>) -> Result<(), AppError> {
        if let Some(sid) = session_id {
            let _ = self.repo.delete_session(&sid).await;
        }

        let _ = self
            .repo
            .create_security_audit_log(Some(user_id), audit_events::LOGOUT, true, client_ip, user_agent, None)
            .await;

        Ok(())
    }
}
