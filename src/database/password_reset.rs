use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::models::password_reset::{PasswordReset, SecurityAuditLog};
use chrono::{DateTime, Utc};
use rand::Rng;
use serde_json::Value as JsonValue;
use sha2::{Digest, Sha256};
use uuid::Uuid;

impl PostgresRepository {
    /// Generate a cryptographically secure password reset token
    /// Returns: (plain_token, token_hash)
    pub fn generate_reset_token() -> (String, String) {
        let mut rng = rand::thread_rng();
        let token_bytes: [u8; 32] = rng.r#gen();
        let token = hex::encode(token_bytes);

        // Store hash, send plain token via email
        let mut hasher = Sha256::new();
        hasher.update(&token);
        let token_hash = hex::encode(hasher.finalize());

        (token, token_hash)
    }

    /// Create a password reset token in the database
    pub async fn create_password_reset(
        &self,
        user_id: &Uuid,
        token_hash: &str,
        expires_at: DateTime<Utc>,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> Result<PasswordReset, AppError> {
        let reset = sqlx::query_as::<_, PasswordReset>(
            r#"
            INSERT INTO password_resets (user_id, token_hash, expires_at, ip_address, user_agent)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, user_id, token_hash, ip_address, user_agent, created_at, expires_at, used_at
            "#,
        )
        .bind(user_id)
        .bind(token_hash)
        .bind(expires_at)
        .bind(ip_address)
        .bind(user_agent)
        .fetch_one(&self.pool)
        .await?;

        Ok(reset)
    }

    /// Find a password reset by token hash
    pub async fn get_password_reset_by_token(&self, token_hash: &str) -> Result<Option<PasswordReset>, AppError> {
        let reset = sqlx::query_as::<_, PasswordReset>(
            r#"
            SELECT id, user_id, token_hash, ip_address, user_agent, created_at, expires_at, used_at
            FROM password_resets
            WHERE token_hash = $1
            "#,
        )
        .bind(token_hash)
        .fetch_optional(&self.pool)
        .await?;

        Ok(reset)
    }

    /// Mark a password reset token as used
    pub async fn mark_password_reset_used(&self, reset_id: &Uuid) -> Result<(), AppError> {
        sqlx::query(
            r#"
            UPDATE password_resets
            SET used_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(reset_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Delete all password reset tokens for a user (useful after successful reset)
    pub async fn delete_password_resets_for_user(&self, user_id: &Uuid) -> Result<(), AppError> {
        sqlx::query(
            r#"
            DELETE FROM password_resets
            WHERE user_id = $1
            "#,
        )
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Clean up expired password reset tokens
    pub async fn cleanup_expired_password_resets(&self) -> Result<u64, AppError> {
        let result = sqlx::query(
            r#"
            DELETE FROM password_resets
            WHERE expires_at < NOW()
            AND used_at IS NULL
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    /// Count password reset attempts for a user within a time window
    pub async fn count_password_reset_attempts(&self, user_id: &Uuid, since: DateTime<Utc>) -> Result<i64, AppError> {
        let count: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*)
            FROM password_resets
            WHERE user_id = $1
            AND created_at >= $2
            "#,
        )
        .bind(user_id)
        .bind(since)
        .fetch_one(&self.pool)
        .await?;

        Ok(count.0)
    }

    /// Create a security audit log entry
    pub async fn create_security_audit_log(
        &self,
        user_id: Option<&Uuid>,
        event_type: &str,
        success: bool,
        ip_address: Option<String>,
        user_agent: Option<String>,
        metadata: Option<JsonValue>,
    ) -> Result<SecurityAuditLog, AppError> {
        let log = sqlx::query_as::<_, SecurityAuditLog>(
            r#"
            INSERT INTO security_audit_log (user_id, event_type, success, ip_address, user_agent, metadata)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, user_id, event_type, ip_address, user_agent, success, metadata, created_at
            "#,
        )
        .bind(user_id)
        .bind(event_type)
        .bind(success)
        .bind(ip_address)
        .bind(user_agent)
        .bind(metadata)
        .fetch_one(&self.pool)
        .await?;

        Ok(log)
    }

    /// Update user password (used during password reset)
    pub async fn update_user_password(&self, user_id: &Uuid, password: &str) -> Result<(), AppError> {
        let (salt, password_hash) = crate::database::user::password_hash(password);

        sqlx::query(
            r#"
            UPDATE users
            SET salt = $1, password_hash = $2
            WHERE id = $3
            "#,
        )
        .bind(&salt)
        .bind(&password_hash)
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Invalidate all sessions for a user (used after password reset for security)
    pub async fn invalidate_all_user_sessions(&self, user_id: &Uuid) -> Result<u64, AppError> {
        let result = sqlx::query(
            r#"
            DELETE FROM sessions
            WHERE user_id = $1
            "#,
        )
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_reset_token() {
        let (token, token_hash) = PostgresRepository::generate_reset_token();

        // Token should be 64 hex characters (32 bytes * 2)
        assert_eq!(token.len(), 64);
        assert!(token.chars().all(|c| c.is_ascii_hexdigit()));

        // Token hash should also be 64 hex characters (SHA-256 = 32 bytes * 2)
        assert_eq!(token_hash.len(), 64);
        assert!(token_hash.chars().all(|c| c.is_ascii_hexdigit()));

        // Token and hash should be different
        assert_ne!(token, token_hash);

        // Verify hash is correct
        let mut hasher = Sha256::new();
        hasher.update(&token);
        let expected_hash = hex::encode(hasher.finalize());
        assert_eq!(token_hash, expected_hash);
    }

    #[test]
    fn test_generate_reset_token_unique() {
        let (token1, hash1) = PostgresRepository::generate_reset_token();
        let (token2, hash2) = PostgresRepository::generate_reset_token();

        // Each call should produce unique tokens
        assert_ne!(token1, token2);
        assert_ne!(hash1, hash2);
    }
}
