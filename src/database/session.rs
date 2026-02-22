use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::models::session::{Session, SessionInfoFull, SessionUser};
use chrono::{DateTime, Utc};
use uuid::Uuid;

impl PostgresRepository {
    pub async fn create_session(
        &self,
        user_id: &Uuid,
        expires_at: DateTime<Utc>,
        user_agent: Option<&str>,
        ip_address: Option<&str>,
    ) -> Result<Session, AppError> {
        self.delete_expired_sessions_for_user(user_id).await?;

        let session = sqlx::query_as::<_, Session>(
            r#"
            INSERT INTO user_session (user_id, expires_at, user_agent, ip_address)
            VALUES ($1, $2, $3, $4)
            RETURNING id
            "#,
        )
        .bind(user_id)
        .bind(expires_at)
        .bind(user_agent)
        .bind(ip_address)
        .fetch_one(&self.pool)
        .await?;

        Ok(session)
    }

    pub async fn get_active_session_user(&self, session_id: &Uuid, user_id: &Uuid) -> Result<Option<SessionUser>, AppError> {
        let user = sqlx::query_as::<_, SessionUser>(
            r#"
            SELECT u.id, u.email
            FROM user_session s
            JOIN users u ON u.id = s.user_id
            WHERE s.id = $1
              AND s.user_id = $2
              AND s.expires_at > now()
            "#,
        )
        .bind(session_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }

    pub async fn delete_session_if_expired(&self, session_id: &Uuid) -> Result<(), AppError> {
        sqlx::query("DELETE FROM user_session WHERE id = $1 AND expires_at <= now()")
            .bind(session_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn delete_expired_sessions_for_user(&self, user_id: &Uuid) -> Result<(), AppError> {
        sqlx::query("DELETE FROM user_session WHERE user_id = $1 AND expires_at <= now()")
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn delete_session(&self, session_id: &Uuid) -> Result<(), AppError> {
        sqlx::query("DELETE FROM user_session WHERE id = $1")
            .bind(session_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn list_sessions_for_user(&self, user_id: &Uuid) -> Result<Vec<SessionInfoFull>, AppError> {
        let sessions = sqlx::query_as::<_, SessionInfoFull>(
            r#"
            SELECT id, created_at, expires_at, user_agent
            FROM user_session
            WHERE user_id = $1 AND expires_at > now()
            ORDER BY created_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(sessions)
    }

    /// Deletes a session only if it belongs to the given user.
    /// Returns `NotFound` if the session does not exist or belongs to a different user.
    pub async fn delete_session_for_user(&self, session_id: &Uuid, user_id: &Uuid) -> Result<(), AppError> {
        let rows = sqlx::query("DELETE FROM user_session WHERE id = $1 AND user_id = $2")
            .bind(session_id)
            .bind(user_id)
            .execute(&self.pool)
            .await?
            .rows_affected();

        if rows == 0 {
            return Err(AppError::NotFound("Session not found".to_string()));
        }

        Ok(())
    }
}
