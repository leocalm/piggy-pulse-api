use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::models::api_token::ApiToken;
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(sqlx::FromRow)]
struct ApiTokenRow {
    id: Uuid,
    user_id: Uuid,
    access_token_hash: String,
    refresh_token_hash: String,
    device_name: Option<String>,
    device_id: Option<String>,
    expires_at: DateTime<Utc>,
    refresh_expires_at: DateTime<Utc>,
    last_used_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    revoked_at: Option<DateTime<Utc>>,
}

impl From<ApiTokenRow> for ApiToken {
    fn from(row: ApiTokenRow) -> Self {
        Self {
            id: row.id,
            user_id: row.user_id,
            access_token_hash: row.access_token_hash,
            refresh_token_hash: row.refresh_token_hash,
            device_name: row.device_name,
            device_id: row.device_id,
            expires_at: row.expires_at,
            refresh_expires_at: row.refresh_expires_at,
            last_used_at: row.last_used_at,
            created_at: row.created_at,
            revoked_at: row.revoked_at,
        }
    }
}

#[allow(dead_code)]
impl PostgresRepository {
    #[allow(clippy::too_many_arguments)]
    pub async fn create_api_token(
        &self,
        user_id: &Uuid,
        access_hash: String,
        refresh_hash: String,
        device_name: String,
        device_id: &str,
        expires_at: &DateTime<Utc>,
        refresh_expires_at: &DateTime<Utc>,
    ) -> Result<ApiToken, AppError> {
        let row = sqlx::query_as::<_, ApiTokenRow>(
            r#"
            INSERT INTO api_tokens (user_id, access_token_hash, refresh_token_hash, device_name, device_id, expires_at, refresh_expires_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (user_id, device_id) DO UPDATE SET
                access_token_hash = EXCLUDED.access_token_hash,
                refresh_token_hash = EXCLUDED.refresh_token_hash,
                device_name = EXCLUDED.device_name,
                device_id = EXCLUDED.device_id,
                expires_at = EXCLUDED.expires_at,
                refresh_expires_at = EXCLUDED.refresh_expires_at
            RETURNING
                id,
                user_id,
                access_token_hash,
                refresh_token_hash,
                device_name,
                device_id,
                expires_at,
                refresh_expires_at,
                last_used_at,
                created_at,
                revoked_at
            "#,
        )
        .bind(user_id)
        .bind(access_hash)
        .bind(refresh_hash)
        .bind(device_name)
        .bind(device_id)
        .bind(expires_at)
        .bind(refresh_expires_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.into())
    }

    pub async fn find_by_access_hash(&self, access_hash: &str) -> Result<Option<ApiToken>, AppError> {
        let row = sqlx::query_as::<_, ApiTokenRow>(
            r#"
            SELECT
                id,
                user_id,
                access_token_hash,
                refresh_token_hash,
                device_name,
                device_id,
                expires_at,
                refresh_expires_at,
                last_used_at,
                created_at,
                revoked_at
            FROM api_tokens
            WHERE access_token_hash = $1 AND revoked_at IS NULL"#,
        )
        .bind(access_hash)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row { Ok(Some(row.into())) } else { Ok(None) }
    }

    pub async fn find_by_refresh_hash(&self, refresh_hash: &str) -> Result<Option<ApiToken>, AppError> {
        let row = sqlx::query_as::<_, ApiTokenRow>(
            r#"
            SELECT
                id,
                user_id,
                access_token_hash,
                refresh_token_hash,
                device_name,
                device_id,
                expires_at,
                refresh_expires_at,
                last_used_at,
                created_at,
                revoked_at
            FROM api_tokens
            WHERE refresh_token_hash = $1 AND revoked_at IS NULL"#,
        )
        .bind(refresh_hash)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row { Ok(Some(row.into())) } else { Ok(None) }
    }

    pub async fn find_by_user(&self, user_id: &Uuid) -> Result<Vec<ApiToken>, AppError> {
        let rows = sqlx::query_as::<_, ApiTokenRow>(
            r#"
                SELECT
                    id,
                    user_id,
                    access_token_hash,
                    refresh_token_hash,
                    device_name,
                    device_id,
                    expires_at,
                    refresh_expires_at,
                    last_used_at,
                    created_at,
                    revoked_at
                FROM api_tokens
                WHERE user_id = $1 AND revoked_at IS NULL"#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(ApiToken::from).collect())
    }

    pub async fn touch(&self, id: &Uuid) -> Result<(), AppError> {
        sqlx::query(
            r#"
            UPDATE api_tokens
            SET last_used_at = NOW()
            WHERE id = $1"#,
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn revoke(&self, id: &Uuid) -> Result<(), AppError> {
        sqlx::query(
            r#"
            UPDATE api_tokens
            SET revoked_at = NOW()
            WHERE id = $1"#,
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn revoke_all_for_user(&self, user_id: &Uuid) -> Result<(), AppError> {
        sqlx::query(
            r#"
            UPDATE api_tokens
            SET revoked_at = NOW()
            WHERE user_id = $1 AND revoked_at IS NULL"#,
        )
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn update_access_token(&self, id: &Uuid, new_hash: String, new_expires_at: &DateTime<Utc>) -> Result<(), AppError> {
        sqlx::query(
            r#"
            UPDATE api_tokens
            SET access_token_hash = $1, expires_at = $2
            WHERE id = $3 AND revoked_at IS NULL"#,
        )
        .bind(new_hash)
        .bind(new_expires_at)
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn cleanup_expired(&self) -> Result<(), AppError> {
        sqlx::query(
            r#"
            DELETE FROM api_tokens
            WHERE refresh_expires_at < NOW()
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
