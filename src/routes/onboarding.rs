use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::middleware::rate_limit::RateLimit;
use crate::models::onboarding::{OnboardingStatus, OnboardingStatusResponse, OnboardingStep};
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{State, get, post};
use rocket_okapi::openapi;
use sqlx::PgPool;
use uuid::Uuid;

/// Derive the current onboarding step for a user based on their data.
/// Returns `None` only if the user's status is already `completed`.
async fn derive_current_step(db: &PgPool, user_id: &Uuid) -> Result<Option<OnboardingStep>, AppError> {
    // Check if a period schedule exists for the user
    let has_period: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM period_schedule WHERE user_id = $1)")
        .bind(user_id)
        .fetch_one(db)
        .await
        .map_err(AppError::from)?;

    if !has_period {
        return Ok(Some(OnboardingStep::Period));
    }

    // Check if any non-archived accounts exist for the user
    let has_accounts: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM account WHERE user_id = $1 AND is_archived = FALSE)")
        .bind(user_id)
        .fetch_one(db)
        .await
        .map_err(AppError::from)?;

    if !has_accounts {
        return Ok(Some(OnboardingStep::Accounts));
    }

    // Check if at least 1 Incoming AND 1 Outgoing non-archived non-system category exists
    let has_incoming: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM category WHERE user_id = $1 AND is_archived = FALSE AND is_system = FALSE AND category_type = 'Incoming'::category_type)",
    )
    .bind(user_id)
    .fetch_one(db)
    .await
    .map_err(AppError::from)?;

    let has_outgoing: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM category WHERE user_id = $1 AND is_archived = FALSE AND is_system = FALSE AND category_type = 'Outgoing'::category_type)",
    )
    .bind(user_id)
    .fetch_one(db)
    .await
    .map_err(AppError::from)?;

    if !has_incoming || !has_outgoing {
        return Ok(Some(OnboardingStep::Categories));
    }

    // All steps done — show summary
    Ok(Some(OnboardingStep::Summary))
}

/// Get the authenticated user's onboarding status and current step
#[openapi(tag = "Onboarding")]
#[get("/status")]
pub async fn get_status(pool: &State<PgPool>, _rate_limit: RateLimit, current_user: CurrentUser) -> Result<Json<OnboardingStatusResponse>, AppError> {
    let db = pool.inner();

    // 1. Check onboarding_status on the users row
    let onboarding_status: String = sqlx::query_scalar("SELECT onboarding_status FROM users WHERE id = $1")
        .bind(current_user.id)
        .fetch_one(db)
        .await
        .map_err(AppError::from)?;

    if onboarding_status == "completed" {
        return Ok(Json(OnboardingStatusResponse {
            status: OnboardingStatus::Completed,
            current_step: None,
        }));
    }

    // Always derive the current step from data so that refreshing the page
    // resumes at the correct step, even when onboarding_status is 'not_started'
    // (the DB column is only written at completion, never for intermediate steps).
    let current_step = derive_current_step(db, &current_user.id).await?;

    // If the user has made no progress at all (period not yet configured),
    // report not_started. Otherwise report in_progress so the frontend
    // can navigate to the correct step.
    let status = if matches!(current_step, Some(OnboardingStep::Period)) {
        OnboardingStatus::NotStarted
    } else {
        OnboardingStatus::InProgress
    };

    Ok(Json(OnboardingStatusResponse { status, current_step }))
}

/// Mark onboarding as completed for the authenticated user
#[openapi(tag = "Onboarding")]
#[post("/complete")]
pub async fn post_complete(pool: &State<PgPool>, _rate_limit: RateLimit, current_user: CurrentUser) -> Result<Status, AppError> {
    let db = pool.inner();

    // Check onboarding_status on the users row
    let onboarding_status: String = sqlx::query_scalar("SELECT onboarding_status FROM users WHERE id = $1")
        .bind(current_user.id)
        .fetch_one(db)
        .await
        .map_err(AppError::from)?;

    // Already completed — idempotent success
    if onboarding_status == "completed" {
        return Ok(Status::NoContent);
    }

    // Verify all steps are complete before marking as completed
    let current_step = derive_current_step(db, &current_user.id).await?;
    if !matches!(current_step, Some(OnboardingStep::Summary)) {
        return Err(AppError::BadRequest("Onboarding steps are not yet complete".to_string()));
    }

    sqlx::query("UPDATE users SET onboarding_status = 'completed' WHERE id = $1")
        .bind(current_user.id)
        .execute(db)
        .await
        .map_err(AppError::from)?;

    // Eagerly generate the first budget periods so the user lands on a
    // populated dashboard rather than waiting for the next cron run.
    let repo = PostgresRepository { pool: db.clone() };
    repo.generate_automatic_budget_periods().await?;

    Ok(Status::NoContent)
}

pub fn routes() -> (Vec<rocket::Route>, okapi::openapi3::OpenApi) {
    rocket_okapi::openapi_get_routes_spec![get_status, post_complete]
}

#[cfg(test)]
mod tests {
    use crate::{Config, build_rocket};
    use rocket::http::Status;
    use rocket::local::asynchronous::Client;

    fn test_config() -> Config {
        let mut config = Config::default();
        config.database.url = "postgres://postgres:example@127.0.0.1:5432/piggy_pulse_db".to_string(); // pragma: allowlist secret
        config.session.cookie_secure = false;
        config
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn get_status_unauthenticated_returns_401() {
        let client = Client::tracked(build_rocket(test_config())).await.expect("valid rocket instance");
        let response = client.get("/api/v1/onboarding/status").dispatch().await;
        assert_eq!(response.status(), Status::Unauthorized);
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn complete_unauthenticated_returns_401() {
        let client = Client::tracked(build_rocket(test_config())).await.expect("valid rocket instance");
        let response = client.post("/api/v1/onboarding/complete").dispatch().await;
        assert_eq!(response.status(), Status::Unauthorized);
    }
}
