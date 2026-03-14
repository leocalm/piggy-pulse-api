// src/service/two_factor.rs

use crate::Config;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::auth::{TwoFactorEnableResponse, TwoFactorStatusResponse};
use crate::error::app_error::AppError;
use crate::models::audit::audit_events;
use crate::models::user::User;
use crate::service::auth::AuthService;
use chrono::Utc;
use sha2::{Digest, Sha256};
use uuid::Uuid;

#[allow(dead_code)]
pub struct TwoFactorService<'a> {
    pub repo: &'a PostgresRepository,
    pub config: &'a Config,
}

#[allow(dead_code)]
impl<'a> TwoFactorService<'a> {
    pub fn new(repo: &'a PostgresRepository, config: &'a Config) -> Self {
        TwoFactorService { repo, config }
    }

    /// Get the current 2FA status for a user.
    pub async fn get_status(&self, user_id: &Uuid) -> Result<TwoFactorStatusResponse, AppError> {
        let two_factor = self.repo.get_two_factor_by_user(user_id).await?;
        let enabled = two_factor.map(|tf| tf.is_enabled).unwrap_or(false);

        let (has_backup_codes, backup_codes_remaining) = if enabled {
            let count = self.repo.count_unused_backup_codes(user_id).await?;
            (count > 0, count as u32)
        } else {
            (false, 0)
        };

        Ok(TwoFactorStatusResponse {
            enabled,
            has_backup_codes,
            backup_codes_remaining,
        })
    }

    /// Initialize 2FA setup: generate secret, QR code, and backup codes.
    /// Returns the secret and QR code data URL.
    pub async fn enable(&self, user_id: &Uuid, username: &str) -> Result<TwoFactorEnableResponse, AppError> {
        // Check if already enabled
        if let Some(existing) = self.repo.get_two_factor_by_user(user_id).await?
            && existing.is_enabled
        {
            return Err(AppError::UserAlreadyExists("Two-factor authentication is already enabled.".to_string()));
        }

        let encryption_key = self.config.two_factor.parse_encryption_key().map_err(AppError::BadRequest)?;

        let issuer_name = self.config.two_factor.issuer_name.clone();
        let username_owned = username.to_string();
        let (secret, encrypted_secret, nonce, qr_code) = tokio::task::spawn_blocking(move || {
            let secret = PostgresRepository::generate_totp_secret();
            let (encrypted_secret, nonce) = PostgresRepository::encrypt_secret(&secret, &encryption_key)?;
            let qr_code = PostgresRepository::generate_qr_code(&secret, &issuer_name, &username_owned)?;
            Ok::<_, AppError>((secret, encrypted_secret, nonce, qr_code))
        })
        .await
        .map_err(|e| AppError::BadRequest(format!("Task join error: {}", e)))??;

        // Store encrypted secret (not yet enabled)
        self.repo.create_two_factor_setup(user_id, &encrypted_secret, &nonce).await?;

        // Generate backup codes (stored but not returned in V2 enable response)
        let _ = self.repo.generate_backup_codes(user_id).await?;

        Ok(TwoFactorEnableResponse { secret, qr_code_uri: qr_code })
    }

    /// Verify a TOTP code and enable 2FA for the user.
    pub async fn verify_setup(&self, user_id: &Uuid, code: &str, client_ip: Option<String>, user_agent: Option<String>) -> Result<(), AppError> {
        let two_factor = self
            .repo
            .get_two_factor_by_user(user_id)
            .await?
            .ok_or_else(|| AppError::BadRequest("Two-factor authentication setup not found. Please initialize setup first.".to_string()))?;

        if two_factor.is_enabled {
            return Err(AppError::BadRequest("Two-factor authentication is already enabled.".to_string()));
        }

        let encryption_key = self.config.two_factor.parse_encryption_key().map_err(AppError::BadRequest)?;
        let secret = PostgresRepository::decrypt_secret(&two_factor.encrypted_secret, &two_factor.encryption_nonce, &encryption_key)?;

        let is_valid = PostgresRepository::verify_totp_code(&secret, code)?;
        if !is_valid {
            return Err(AppError::BadRequest("Invalid verification code.".to_string()));
        }

        // Enable 2FA
        self.repo.verify_and_enable_two_factor(user_id).await?;

        // Revoke all API tokens
        let _ = self.repo.revoke_all_for_user(user_id).await;

        let _ = self
            .repo
            .create_security_audit_log(Some(user_id), audit_events::TWO_FACTOR_ENABLED, true, client_ip, user_agent, None)
            .await;

        Ok(())
    }

    /// Complete a 2FA login challenge: verify token + TOTP code, create session.
    /// Returns (user, session_id).
    pub async fn verify_login(
        &self,
        two_factor_token: &str,
        code: &str,
        client_ip: Option<String>,
        user_agent: Option<String>,
    ) -> Result<(User, Uuid), AppError> {
        // Hash the incoming token to look up in DB
        let token_hash = hex::encode(Sha256::digest(two_factor_token.as_bytes()));

        let pending = self.repo.take_pending_2fa_token(&token_hash).await?.ok_or(AppError::Unauthorized)?;

        if pending.expires_at <= Utc::now() {
            return Err(AppError::Unauthorized);
        }

        // Verify the TOTP/backup code
        let two_factor_data = self.repo.get_two_factor_by_user(&pending.user_id).await?.ok_or(AppError::Unauthorized)?;

        let auth = AuthService::new(self.repo, self.config);
        let backup_code_used = auth
            .verify_two_factor(&pending.user_id, two_factor_data, code, client_ip.clone(), user_agent.clone())
            .await?;

        if backup_code_used {
            let _ = self
                .repo
                .create_security_audit_log(
                    Some(&pending.user_id),
                    audit_events::TWO_FACTOR_BACKUP_USED,
                    true,
                    client_ip.clone(),
                    user_agent.clone(),
                    None,
                )
                .await;
        }

        // Fetch the user
        let user = self.repo.get_user_by_id(&pending.user_id).await?.ok_or(AppError::Unauthorized)?;

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
                Some(serde_json::json!({"2fa_used": true})),
            )
            .await;

        Ok((user, session.id))
    }

    /// Disable 2FA for a user (V2: requires only a TOTP/backup code, not password).
    pub async fn disable(&self, user_id: &Uuid, code: &str, client_ip: Option<String>, user_agent: Option<String>) -> Result<(), AppError> {
        let two_factor = self
            .repo
            .get_two_factor_by_user(user_id)
            .await?
            .ok_or_else(|| AppError::BadRequest("Two-factor authentication is not enabled.".to_string()))?;

        if !two_factor.is_enabled {
            return Err(AppError::BadRequest("Two-factor authentication is not enabled.".to_string()));
        }

        let encryption_key = self.config.two_factor.parse_encryption_key().map_err(AppError::BadRequest)?;
        let secret = PostgresRepository::decrypt_secret(&two_factor.encrypted_secret, &two_factor.encryption_nonce, &encryption_key)?;

        // Verify TOTP or backup code
        let totp_valid = PostgresRepository::verify_totp_code(&secret, code)?;
        let backup_valid = if !totp_valid {
            self.repo.verify_backup_code(user_id, code).await?
        } else {
            false
        };

        if !totp_valid && !backup_valid {
            return Err(AppError::BadRequest("Invalid two-factor code.".to_string()));
        }

        // Disable 2FA
        self.repo.disable_two_factor(user_id).await?;

        // Revoke all API tokens
        let _ = self.repo.revoke_all_for_user(user_id).await;

        let _ = self
            .repo
            .create_security_audit_log(
                Some(user_id),
                audit_events::TWO_FACTOR_DISABLED,
                true,
                client_ip,
                user_agent,
                Some(serde_json::json!({"method": "normal"})),
            )
            .await;

        Ok(())
    }

    /// Regenerate backup codes (requires a valid TOTP/backup code).
    pub async fn regenerate_backup_codes(&self, user_id: &Uuid, code: &str) -> Result<Vec<String>, AppError> {
        let two_factor = self
            .repo
            .get_two_factor_by_user(user_id)
            .await?
            .ok_or_else(|| AppError::BadRequest("Two-factor authentication is not enabled.".to_string()))?;

        if !two_factor.is_enabled {
            return Err(AppError::BadRequest("Two-factor authentication is not enabled.".to_string()));
        }

        let encryption_key = self.config.two_factor.parse_encryption_key().map_err(AppError::BadRequest)?;

        // Decrypt and verify in blocking task
        let encrypted_secret = two_factor.encrypted_secret.clone();
        let encryption_nonce = two_factor.encryption_nonce.clone();
        let code_owned = code.to_string();
        tokio::task::spawn_blocking(move || {
            let secret = PostgresRepository::decrypt_secret(&encrypted_secret, &encryption_nonce, &encryption_key)?;
            let is_valid = PostgresRepository::verify_totp_code(&secret, &code_owned)?;
            if !is_valid {
                return Err(AppError::BadRequest("Invalid two-factor code.".to_string()));
            }
            Ok::<_, AppError>(())
        })
        .await
        .map_err(|e| AppError::BadRequest(format!("Task join error: {}", e)))??;

        let backup_codes = self.repo.generate_backup_codes(user_id).await?;
        Ok(backup_codes)
    }

    /// Request emergency 2FA disable via email. Always returns Ok (anti-enumeration).
    pub async fn emergency_disable_request(&self, email: &str) -> Result<(), AppError> {
        let user = match self.repo.get_user_by_email(email).await? {
            Some(u) => u,
            None => return Ok(()),
        };

        let has_2fa = self.repo.get_two_factor_by_user(&user.id).await?.map(|tf| tf.is_enabled).unwrap_or(false);

        if !has_2fa {
            return Ok(());
        }

        let token = self.repo.create_emergency_token(&user.id).await?;

        let email_service = crate::service::email::EmailService::new(self.config.email.clone());
        if let Err(e) = email_service
            .send_emergency_2fa_disable_email(&user.email, &user.name, &token, &self.config.two_factor.frontend_emergency_disable_url)
            .await
        {
            tracing::error!("Failed to send emergency 2FA disable email to {}: {}", user.email, e);
        }

        Ok(())
    }

    /// Confirm emergency 2FA disable with a token from the email.
    pub async fn emergency_disable_confirm(&self, token: &str, client_ip: Option<String>, user_agent: Option<String>) -> Result<(), AppError> {
        let user_id = self
            .repo
            .verify_emergency_token(token)
            .await?
            .ok_or_else(|| AppError::BadRequest("Invalid or expired emergency disable token.".to_string()))?;

        self.repo.disable_two_factor(&user_id).await?;

        let _ = self
            .repo
            .create_security_audit_log(
                Some(&user_id),
                audit_events::TWO_FACTOR_DISABLED,
                true,
                client_ip,
                user_agent,
                Some(serde_json::json!({"method": "emergency"})),
            )
            .await;

        Ok(())
    }
}
