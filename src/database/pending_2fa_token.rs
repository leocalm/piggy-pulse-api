use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[allow(dead_code)]
pub struct PendingTwoFaToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub device_name: String,
    pub device_id: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
#[allow(dead_code)]
struct PendingRow {
    id: Uuid,
    user_id: Uuid,
    device_name: String,
    device_id: String,
    expires_at: DateTime<Utc>,
}

#[allow(dead_code)]
impl PostgresRepository {
    pub async fn create_pending_2fa_token(
        &self,
        user_id: &Uuid,
        token_hash: &str,
        device_name: &str,
        device_id: &str,
        expires_at: &DateTime<Utc>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO pending_2fa_tokens (user_id, token_hash, device_name, device_id, expires_at)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(user_id)
        .bind(token_hash)
        .bind(device_name)
        .bind(device_id)
        .bind(expires_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn take_pending_2fa_token(&self, token_hash: &str) -> Result<Option<PendingTwoFaToken>, AppError> {
        let row = sqlx::query_as::<_, PendingRow>(
            "DELETE FROM pending_2fa_tokens WHERE token_hash = $1
             RETURNING id, user_id, device_name, device_id, expires_at",
        )
        .bind(token_hash)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| PendingTwoFaToken {
            id: r.id,
            user_id: r.user_id,
            device_name: r.device_name,
            device_id: r.device_id,
            expires_at: r.expires_at,
        }))
    }

    pub async fn cleanup_expired_pending_2fa_tokens(&self) -> Result<(), AppError> {
        sqlx::query("DELETE FROM pending_2fa_tokens WHERE expires_at < NOW()")
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
