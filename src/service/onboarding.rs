use crate::database::postgres_repository::PostgresRepository;
use crate::dto::misc::{OnboardingStatus, OnboardingStatusResponse, OnboardingStep};
use crate::error::app_error::AppError;
use uuid::Uuid;

pub struct OnboardingService<'a> {
    repository: &'a PostgresRepository,
}

impl<'a> OnboardingService<'a> {
    pub fn new(repository: &'a PostgresRepository) -> Self {
        OnboardingService { repository }
    }

    pub async fn get_status(&self, user_id: &Uuid) -> Result<OnboardingStatusResponse, AppError> {
        let onboarding_status: String = sqlx::query_scalar("SELECT onboarding_status FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_one(&self.repository.pool)
            .await
            .map_err(AppError::from)?;

        if onboarding_status == "completed" {
            return Ok(OnboardingStatusResponse {
                status: OnboardingStatus::Completed,
                current_step: None,
            });
        }

        let current_step = self.derive_current_step(user_id).await?;

        let status = if matches!(current_step, Some(OnboardingStep::Period)) {
            OnboardingStatus::NotStarted
        } else {
            OnboardingStatus::InProgress
        };

        Ok(OnboardingStatusResponse { status, current_step })
    }

    pub async fn complete(&self, user_id: &Uuid) -> Result<(), AppError> {
        let onboarding_status: String = sqlx::query_scalar("SELECT onboarding_status FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_one(&self.repository.pool)
            .await
            .map_err(AppError::from)?;

        if onboarding_status == "completed" {
            self.repository.generate_automatic_budget_periods().await?;
            return Ok(());
        }

        let current_step = self.derive_current_step(user_id).await?;
        if !matches!(current_step, Some(OnboardingStep::Summary)) {
            return Err(AppError::BadRequest("Onboarding steps are not yet complete".to_string()));
        }

        sqlx::query("UPDATE users SET onboarding_status = 'completed' WHERE id = $1")
            .bind(user_id)
            .execute(&self.repository.pool)
            .await
            .map_err(AppError::from)?;

        self.repository.generate_automatic_budget_periods().await?;

        Ok(())
    }

    async fn derive_current_step(&self, user_id: &Uuid) -> Result<Option<OnboardingStep>, AppError> {
        let has_period: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM period_schedule WHERE user_id = $1)")
            .bind(user_id)
            .fetch_one(&self.repository.pool)
            .await
            .map_err(AppError::from)?;

        if !has_period {
            return Ok(Some(OnboardingStep::Period));
        }

        let has_accounts: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM account WHERE user_id = $1 AND is_archived = FALSE)")
            .bind(user_id)
            .fetch_one(&self.repository.pool)
            .await
            .map_err(AppError::from)?;

        if !has_accounts {
            return Ok(Some(OnboardingStep::Accounts));
        }

        let has_incoming: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM category WHERE user_id = $1 AND is_archived = FALSE AND is_system = FALSE AND category_type = 'Incoming'::category_type)",
        )
        .bind(user_id)
        .fetch_one(&self.repository.pool)
        .await
        .map_err(AppError::from)?;

        let has_outgoing: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM category WHERE user_id = $1 AND is_archived = FALSE AND is_system = FALSE AND category_type = 'Outgoing'::category_type)",
        )
        .bind(user_id)
        .fetch_one(&self.repository.pool)
        .await
        .map_err(AppError::from)?;

        if !has_incoming || !has_outgoing {
            return Ok(Some(OnboardingStep::Categories));
        }

        Ok(Some(OnboardingStep::Summary))
    }
}
