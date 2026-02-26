use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::models::settings::{
    PeriodModelRequest, PeriodModelResponse, PeriodSchedule, ProfileData, ProfileRequest, ScheduleConfigResponse, Settings, SettingsRequest, UserPreferences,
};
use uuid::Uuid;

impl PostgresRepository {
    pub async fn get_settings(&self, user_id: &Uuid) -> Result<Settings, AppError> {
        let settings = sqlx::query_as::<_, Settings>(
            r#"
            SELECT id, user_id, theme, language, default_currency_id,
                   budget_stability_tolerance_basis_points,
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
            INSERT INTO settings (
                user_id,
                theme,
                language,
                default_currency_id,
                budget_stability_tolerance_basis_points
            )
            VALUES ($1, $2, $3, $4, COALESCE($5, 1000))
            ON CONFLICT (user_id)
            DO UPDATE SET
                theme = EXCLUDED.theme,
                language = EXCLUDED.language,
                default_currency_id = EXCLUDED.default_currency_id,
                budget_stability_tolerance_basis_points = COALESCE(
                    $5,
                    settings.budget_stability_tolerance_basis_points
                ),
                updated_at = now()
            RETURNING id, user_id, theme, language, default_currency_id,
                      budget_stability_tolerance_basis_points,
                      created_at, updated_at
            "#,
        )
        .bind(user_id)
        .bind(&request.theme)
        .bind(&request.language)
        .bind(request.default_currency_id)
        .bind(request.budget_stability_tolerance_basis_points)
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
            INSERT INTO settings (
                user_id,
                theme,
                language,
                default_currency_id,
                budget_stability_tolerance_basis_points
            )
            VALUES ($1, 'light', 'en', NULL, 1000)
            RETURNING id, user_id, theme, language, default_currency_id,
                      budget_stability_tolerance_basis_points,
                      created_at, updated_at
            "#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(settings)
    }

    // ── Profile ───────────────────────────────────────────────────────────────

    pub async fn get_profile(&self, user_id: &Uuid) -> Result<ProfileData, AppError> {
        let profile = sqlx::query_as::<_, ProfileData>(
            r#"
            SELECT u.name, u.email, s.timezone, s.default_currency_id
            FROM users u
            JOIN settings s ON s.user_id = u.id
            WHERE u.id = $1
            "#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(profile)
    }

    pub async fn update_profile(&self, user_id: &Uuid, request: &ProfileRequest) -> Result<ProfileData, AppError> {
        let mut tx = self.pool.begin().await?;

        sqlx::query("UPDATE users SET name = $1 WHERE id = $2")
            .bind(&request.name)
            .bind(user_id)
            .execute(&mut *tx)
            .await?;

        sqlx::query("UPDATE settings SET timezone = $1, default_currency_id = $2, updated_at = now() WHERE user_id = $3")
            .bind(&request.timezone)
            .bind(request.default_currency_id)
            .bind(user_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;

        self.get_profile(user_id).await
    }

    // ── Preferences ───────────────────────────────────────────────────────────

    pub async fn get_preferences(&self, user_id: &Uuid) -> Result<UserPreferences, AppError> {
        let prefs = sqlx::query_as::<_, UserPreferences>(
            r#"
            SELECT theme, date_format, number_format, compact_mode
            FROM settings
            WHERE user_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(prefs)
    }

    pub async fn update_preferences(
        &self,
        user_id: &Uuid,
        theme: &str,
        date_format: &str,
        number_format: &str,
        compact_mode: bool,
    ) -> Result<UserPreferences, AppError> {
        let prefs = sqlx::query_as::<_, UserPreferences>(
            r#"
            UPDATE settings
            SET theme = $1, date_format = $2, number_format = $3, compact_mode = $4, updated_at = now()
            WHERE user_id = $5
            RETURNING theme, date_format, number_format, compact_mode
            "#,
        )
        .bind(theme)
        .bind(date_format)
        .bind(number_format)
        .bind(compact_mode)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(prefs)
    }

    // ── Period model ──────────────────────────────────────────────────────────

    pub async fn get_period_model(&self, user_id: &Uuid) -> Result<PeriodModelResponse, AppError> {
        let mut tx = self.pool.begin().await?;

        let mode = sqlx::query_scalar::<_, String>("SELECT period_mode FROM settings WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(&mut *tx)
            .await?;

        let schedule = sqlx::query_as::<_, PeriodSchedule>(
            r#"
            SELECT start_day, duration_value, duration_unit,
                   saturday_adjustment, sunday_adjustment, name_pattern, generate_ahead
            FROM period_schedule
            WHERE user_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(PeriodModelResponse {
            mode,
            schedule: schedule.as_ref().map(ScheduleConfigResponse::from),
        })
    }

    pub async fn upsert_period_model(&self, user_id: &Uuid, request: &PeriodModelRequest) -> Result<PeriodModelResponse, AppError> {
        let mut tx = self.pool.begin().await?;

        sqlx::query("UPDATE settings SET period_mode = $1, updated_at = now() WHERE user_id = $2")
            .bind(&request.mode)
            .bind(user_id)
            .execute(&mut *tx)
            .await?;

        if request.mode == "automatic" {
            match &request.schedule {
                Some(s) => {
                    sqlx::query(
                        r#"
                        INSERT INTO period_schedule (
                            user_id, start_day, duration_value, duration_unit,
                            saturday_adjustment, sunday_adjustment, name_pattern, generate_ahead
                        )
                        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                        ON CONFLICT (user_id) DO UPDATE SET
                            start_day = EXCLUDED.start_day,
                            duration_value = EXCLUDED.duration_value,
                            duration_unit = EXCLUDED.duration_unit,
                            saturday_adjustment = EXCLUDED.saturday_adjustment,
                            sunday_adjustment = EXCLUDED.sunday_adjustment,
                            name_pattern = EXCLUDED.name_pattern,
                            generate_ahead = EXCLUDED.generate_ahead,
                            updated_at = now()
                        "#,
                    )
                    .bind(user_id)
                    .bind(s.start_day)
                    .bind(s.duration_value)
                    .bind(&s.duration_unit)
                    .bind(&s.saturday_adjustment)
                    .bind(&s.sunday_adjustment)
                    .bind(&s.name_pattern)
                    .bind(s.generate_ahead)
                    .execute(&mut *tx)
                    .await?;
                }
                None => {
                    tx.rollback().await?;
                    return Err(AppError::BadRequest("schedule is required when mode is 'automatic'".to_string()));
                }
            }
        }

        tx.commit().await?;

        self.get_period_model(user_id).await
    }

    // ── Danger zone ───────────────────────────────────────────────────────────

    /// Removes the user's financial structure: accounts, categories, budget periods,
    /// and period schedule. Cascade rules will also remove transactions and other
    /// dependent data linked to those accounts and categories.
    pub async fn reset_structure(&self, user_id: &Uuid) -> Result<(), AppError> {
        let mut tx = self.pool.begin().await?;

        sqlx::query("DELETE FROM period_schedule WHERE user_id = $1")
            .bind(user_id)
            .execute(&mut *tx)
            .await?;

        sqlx::query("DELETE FROM budget_period WHERE user_id = $1")
            .bind(user_id)
            .execute(&mut *tx)
            .await?;

        sqlx::query("DELETE FROM account WHERE user_id = $1").bind(user_id).execute(&mut *tx).await?;

        sqlx::query("DELETE FROM category WHERE user_id = $1").bind(user_id).execute(&mut *tx).await?;

        tx.commit().await?;

        Ok(())
    }
}
