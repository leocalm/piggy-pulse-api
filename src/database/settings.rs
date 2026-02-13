use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::models::settings::{Settings, SettingsRequest};
use uuid::Uuid;

impl PostgresRepository {
    pub async fn get_settings(&self, user_id: &Uuid) -> Result<Settings, AppError> {
        let settings = sqlx::query_as::<_, Settings>(
            r#"
            SELECT id, user_id, theme, language, default_currency_id,
                   created_at, updated_at
            FROM settings
            WHERE user_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(settings)
    }

    pub async fn upsert_settings(&self, request: &SettingsRequest, user_id: &Uuid) -> Result<Settings, AppError> {
        let mut transaction = self.pool.begin().await?;

        let settings = sqlx::query_as::<_, Settings>(
            r#"
            INSERT INTO settings (user_id, theme, language, default_currency_id)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (user_id)
            DO UPDATE SET
                theme = EXCLUDED.theme,
                language = EXCLUDED.language,
                default_currency_id = EXCLUDED.default_currency_id,
                updated_at = now()
            RETURNING id, user_id, theme, language, default_currency_id,
                      created_at, updated_at
            "#,
        )
        .bind(user_id)
        .bind(&request.theme)
        .bind(&request.language)
        .bind(request.default_currency_id)
        .fetch_one(&mut *transaction)
        .await?;

        if let Some(currency_id) = settings.default_currency_id {
            sqlx::query(
                r#"
                UPDATE account
                SET currency_id = $1
                WHERE user_id = $2
                "#,
            )
            .bind(currency_id)
            .bind(user_id)
            .execute(&mut *transaction)
            .await?;
        }

        transaction.commit().await?;

        Ok(settings)
    }

    pub async fn create_default_settings(&self, user_id: &Uuid) -> Result<Settings, AppError> {
        let settings = sqlx::query_as::<_, Settings>(
            r#"
            INSERT INTO settings (user_id, theme, language, default_currency_id)
            VALUES ($1, 'light', 'en', NULL)
            RETURNING id, user_id, theme, language, default_currency_id,
                      created_at, updated_at
            "#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(settings)
    }
}
