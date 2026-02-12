use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::models::two_factor::{BackupCode, EmergencyToken, TwoFactorAuth, TwoFactorRateLimit};
use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit, OsRng},
};
use argon2::Argon2;
use base64::{Engine as _, engine::general_purpose};
use data_encoding::BASE32_NOPAD;
use password_hash::{PasswordHasher, PasswordVerifier, SaltString};
use qrcode::QrCode;
use sha2::{Digest, Sha256};
use std::time::SystemTime;
use totp_rs::{Algorithm, Secret, TOTP};
use uuid::Uuid;

const TOTP_DIGITS: usize = 6;
const TOTP_STEP: u64 = 30; // 30 seconds
const BACKUP_CODE_LENGTH: usize = 16;
const BACKUP_CODE_COUNT: usize = 10;
const RATE_LIMIT_MAX_ATTEMPTS: i32 = 5;
const RATE_LIMIT_LOCKOUT_MINUTES: i64 = 15;

impl PostgresRepository {
    /// Generate a new TOTP secret (32 characters, base32 encoded)
    pub fn generate_totp_secret() -> String {
        let mut rng = rand::thread_rng();
        let mut secret_bytes = [0u8; 20];
        rand::RngCore::fill_bytes(&mut rng, &mut secret_bytes);
        // Use standard RFC4648 base32 without padding
        BASE32_NOPAD.encode(&secret_bytes)
    }

    /// Encrypt a TOTP secret using AES-256-GCM
    /// Returns (encrypted_base64, nonce_base64)
    pub fn encrypt_secret(secret: &str, key: &[u8; 32]) -> Result<(String, String), AppError> {
        let cipher = Aes256Gcm::new(key.into());
        let mut rng = rand::thread_rng();
        let mut nonce_bytes = [0u8; 12];
        rand::RngCore::fill_bytes(&mut rng, &mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, secret.as_bytes())
            .map_err(|e| AppError::BadRequest(format!("Encryption failed: {}", e)))?;

        let encrypted_base64 = general_purpose::STANDARD.encode(&ciphertext);
        let nonce_base64 = general_purpose::STANDARD.encode(nonce_bytes);

        Ok((encrypted_base64, nonce_base64))
    }

    /// Decrypt a TOTP secret using AES-256-GCM
    pub fn decrypt_secret(encrypted_base64: &str, nonce_base64: &str, key: &[u8; 32]) -> Result<String, AppError> {
        let cipher = Aes256Gcm::new(key.into());

        let ciphertext = general_purpose::STANDARD
            .decode(encrypted_base64)
            .map_err(|e| AppError::BadRequest(format!("Failed to decode encrypted secret: {}", e)))?;

        let nonce_bytes = general_purpose::STANDARD
            .decode(nonce_base64)
            .map_err(|e| AppError::BadRequest(format!("Failed to decode nonce: {}", e)))?;

        let nonce = Nonce::from_slice(&nonce_bytes);

        let plaintext = cipher
            .decrypt(nonce, ciphertext.as_ref())
            .map_err(|e| AppError::BadRequest(format!("Decryption failed: {}", e)))?;

        String::from_utf8(plaintext).map_err(|e| AppError::BadRequest(format!("Invalid UTF-8 in decrypted secret: {}", e)))
    }

    /// Generate a QR code data URL for Google Authenticator
    pub fn generate_qr_code(secret: &str, issuer: &str, account_name: &str) -> Result<String, AppError> {
        // Generate otpauth:// URI for the QR code
        let uri = format!(
            "otpauth://totp/{}:{}?secret={}&issuer={}&algorithm=SHA1&digits={}&period={}",
            urlencoding::encode(issuer),
            urlencoding::encode(account_name),
            secret,
            urlencoding::encode(issuer),
            TOTP_DIGITS,
            TOTP_STEP
        );

        let qr = QrCode::new(&uri).map_err(|e| AppError::BadRequest(format!("Failed to generate QR code: {}", e)))?;

        // Render QR code as SVG (simpler than PNG, no extra dependencies)
        let qr_svg = qr.render::<qrcode::render::svg::Color>().min_dimensions(200, 200).build();

        // Convert SVG to base64 data URL
        let base64_svg = general_purpose::STANDARD.encode(qr_svg.as_bytes());
        Ok(format!("data:image/svg+xml;base64,{}", base64_svg))
    }

    /// Verify a TOTP code with time skew tolerance
    pub fn verify_totp_code(secret: &str, code: &str) -> Result<bool, AppError> {
        let secret_bytes = Secret::Encoded(secret.to_string())
            .to_bytes()
            .map_err(|e| AppError::BadRequest(format!("Failed to decode secret: {}", e)))?;

        let totp =
            TOTP::new(Algorithm::SHA1, TOTP_DIGITS, 1, TOTP_STEP, secret_bytes).map_err(|e| AppError::BadRequest(format!("Failed to create TOTP: {}", e)))?;

        // Use constant-time comparison through totp-rs
        Ok(totp.check(code, SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs()))
    }

    /// Create initial 2FA setup (unverified state)
    pub async fn create_two_factor_setup(&self, user_id: &Uuid, encrypted_secret: &str, nonce: &str) -> Result<TwoFactorAuth, AppError> {
        // Delete any existing 2FA setup for this user (they're starting fresh)
        sqlx::query("DELETE FROM two_factor_auth WHERE user_id = $1")
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        let two_factor = sqlx::query_as::<_, TwoFactorAuth>(
            r#"
            INSERT INTO two_factor_auth (user_id, encrypted_secret, encryption_nonce, is_enabled)
            VALUES ($1, $2, $3, false)
            RETURNING id, user_id, encrypted_secret, encryption_nonce, is_enabled, verified_at, created_at, updated_at
            "#,
        )
        .bind(user_id)
        .bind(encrypted_secret)
        .bind(nonce)
        .fetch_one(&self.pool)
        .await?;

        Ok(two_factor)
    }

    /// Enable 2FA after successful verification
    pub async fn verify_and_enable_two_factor(&self, user_id: &Uuid) -> Result<(), AppError> {
        sqlx::query(
            r#"
            UPDATE two_factor_auth
            SET is_enabled = true, verified_at = now(), updated_at = now()
            WHERE user_id = $1
            "#,
        )
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get 2FA configuration for a user
    pub async fn get_two_factor_by_user(&self, user_id: &Uuid) -> Result<Option<TwoFactorAuth>, AppError> {
        let two_factor = sqlx::query_as::<_, TwoFactorAuth>(
            r#"
            SELECT id, user_id, encrypted_secret, encryption_nonce, is_enabled, verified_at, created_at, updated_at
            FROM two_factor_auth
            WHERE user_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(two_factor)
    }

    /// Generate backup codes synchronously (returns codes with hashes)
    pub fn generate_backup_codes_sync() -> Result<Vec<(String, String)>, AppError> {
        use rand::distributions::{Alphanumeric, DistString};

        let mut codes_with_hashes = Vec::new();
        let argon2 = Argon2::default();
        let mut rng = rand::thread_rng();

        for _ in 0..BACKUP_CODE_COUNT {
            // Generate random alphanumeric code
            let code = Alphanumeric.sample_string(&mut rng, BACKUP_CODE_LENGTH);

            // Hash the code before storing
            let salt = SaltString::generate(&mut OsRng);
            let code_hash = argon2
                .hash_password(code.as_bytes(), &salt)
                .map_err(|e| AppError::BadRequest(format!("Failed to hash backup code: {}", e)))?
                .to_string();

            codes_with_hashes.push((code, code_hash));
        }

        Ok(codes_with_hashes)
    }

    /// Generate backup codes (returns plaintext codes, stores hashed versions)
    pub async fn generate_backup_codes(&self, user_id: &Uuid) -> Result<Vec<String>, AppError> {
        // Delete existing backup codes
        sqlx::query("DELETE FROM two_factor_backup_codes WHERE user_id = $1")
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        // Generate codes in blocking task
        let codes_with_hashes = tokio::task::spawn_blocking(PostgresRepository::generate_backup_codes_sync)
            .await
            .map_err(|e| AppError::BadRequest(format!("Task join error: {}", e)))??;

        let mut codes = Vec::new();

        for (code, code_hash) in codes_with_hashes {
            sqlx::query(
                r#"
                INSERT INTO two_factor_backup_codes (user_id, code_hash)
                VALUES ($1, $2)
                "#,
            )
            .bind(user_id)
            .bind(&code_hash)
            .execute(&self.pool)
            .await?;

            codes.push(code);
        }

        Ok(codes)
    }

    /// Verify a backup code and mark it as used
    pub async fn verify_backup_code(&self, user_id: &Uuid, code: &str) -> Result<bool, AppError> {
        let backup_codes = sqlx::query_as::<_, BackupCode>(
            r#"
            SELECT id, user_id, code_hash, used_at, created_at
            FROM two_factor_backup_codes
            WHERE user_id = $1 AND used_at IS NULL
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        // Try to verify against each unused backup code
        for backup_code in backup_codes {
            let parsed_hash = password_hash::PasswordHash::new(&backup_code.code_hash)
                .map_err(|e| AppError::BadRequest(format!("Failed to parse backup code hash: {}", e)))?;

            let argon2 = Argon2::default();
            if argon2.verify_password(code.as_bytes(), &parsed_hash).is_ok() {
                // Mark this code as used
                sqlx::query(
                    r#"
                    UPDATE two_factor_backup_codes
                    SET used_at = now()
                    WHERE id = $1
                    "#,
                )
                .bind(backup_code.id)
                .execute(&self.pool)
                .await?;

                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Count remaining unused backup codes
    pub async fn count_unused_backup_codes(&self, user_id: &Uuid) -> Result<i32, AppError> {
        let count: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM two_factor_backup_codes
            WHERE user_id = $1 AND used_at IS NULL
            "#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(count.0 as i32)
    }

    /// Disable 2FA for a user (deletes all 2FA data)
    pub async fn disable_two_factor(&self, user_id: &Uuid) -> Result<(), AppError> {
        // Delete emergency tokens
        sqlx::query("DELETE FROM two_factor_emergency_tokens WHERE user_id = $1")
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        // Delete backup codes
        sqlx::query("DELETE FROM two_factor_backup_codes WHERE user_id = $1")
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        // Delete rate limit record
        sqlx::query("DELETE FROM two_factor_rate_limits WHERE user_id = $1")
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        // Delete 2FA configuration
        sqlx::query("DELETE FROM two_factor_auth WHERE user_id = $1")
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Check if user is rate limited
    pub async fn check_rate_limit(&self, user_id: &Uuid) -> Result<bool, AppError> {
        let rate_limit = sqlx::query_as::<_, TwoFactorRateLimit>(
            r#"
            SELECT id, user_id, failed_attempts, locked_until, last_attempt_at
            FROM two_factor_rate_limits
            WHERE user_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        match rate_limit {
            Some(limit) => {
                if let Some(locked_until) = limit.locked_until
                    && locked_until > chrono::Utc::now()
                {
                    return Ok(true); // Still locked
                }
                Ok(false)
            }
            None => Ok(false),
        }
    }

    /// Record a failed 2FA attempt and lock if threshold exceeded
    pub async fn record_failed_attempt(&self, user_id: &Uuid) -> Result<(), AppError> {
        let existing = sqlx::query_as::<_, TwoFactorRateLimit>(
            r#"
            SELECT id, user_id, failed_attempts, locked_until, last_attempt_at
            FROM two_factor_rate_limits
            WHERE user_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        match existing {
            Some(limit) => {
                let new_attempts = limit.failed_attempts + 1;
                let locked_until = if new_attempts >= RATE_LIMIT_MAX_ATTEMPTS {
                    Some(chrono::Utc::now() + chrono::Duration::minutes(RATE_LIMIT_LOCKOUT_MINUTES))
                } else {
                    None
                };

                sqlx::query(
                    r#"
                    UPDATE two_factor_rate_limits
                    SET failed_attempts = $1, locked_until = $2, last_attempt_at = now()
                    WHERE user_id = $3
                    "#,
                )
                .bind(new_attempts)
                .bind(locked_until)
                .bind(user_id)
                .execute(&self.pool)
                .await?;
            }
            None => {
                sqlx::query(
                    r#"
                    INSERT INTO two_factor_rate_limits (user_id, failed_attempts, last_attempt_at)
                    VALUES ($1, 1, now())
                    "#,
                )
                .bind(user_id)
                .execute(&self.pool)
                .await?;
            }
        }

        Ok(())
    }

    /// Reset rate limit after successful authentication
    pub async fn reset_rate_limit(&self, user_id: &Uuid) -> Result<(), AppError> {
        sqlx::query("DELETE FROM two_factor_rate_limits WHERE user_id = $1")
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Create emergency disable token
    pub async fn create_emergency_token(&self, user_id: &Uuid) -> Result<String, AppError> {
        // Generate random token (32 bytes = 64 hex chars)
        let token = {
            let mut rng = rand::thread_rng();
            let mut token_bytes = [0u8; 32];
            rand::RngCore::fill_bytes(&mut rng, &mut token_bytes);
            hex::encode(token_bytes)
        };

        // Hash token before storing
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        let token_hash = hex::encode(hasher.finalize());

        let expires_at = chrono::Utc::now() + chrono::Duration::hours(1);

        sqlx::query(
            r#"
            INSERT INTO two_factor_emergency_tokens (user_id, token_hash, expires_at)
            VALUES ($1, $2, $3)
            "#,
        )
        .bind(user_id)
        .bind(&token_hash)
        .bind(expires_at)
        .execute(&self.pool)
        .await?;

        Ok(token)
    }

    /// Verify and use emergency token (returns user_id if valid)
    pub async fn verify_emergency_token(&self, token: &str) -> Result<Option<Uuid>, AppError> {
        // Hash the provided token
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        let token_hash = hex::encode(hasher.finalize());

        let emergency_token = sqlx::query_as::<_, EmergencyToken>(
            r#"
            SELECT id, user_id, token_hash, expires_at, created_at, used_at
            FROM two_factor_emergency_tokens
            WHERE token_hash = $1 AND used_at IS NULL AND expires_at > now()
            "#,
        )
        .bind(&token_hash)
        .fetch_optional(&self.pool)
        .await?;

        match emergency_token {
            Some(token) => {
                // Mark token as used
                sqlx::query(
                    r#"
                    UPDATE two_factor_emergency_tokens
                    SET used_at = now()
                    WHERE id = $1
                    "#,
                )
                .bind(token.id)
                .execute(&self.pool)
                .await?;

                Ok(Some(token.user_id))
            }
            None => Ok(None),
        }
    }
}
