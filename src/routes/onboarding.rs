use crate::auth::CurrentUser;
use crate::error::app_error::AppError;
use crate::models::onboarding::{OnboardingStatus, OnboardingStatusResponse, OnboardingStep};
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{State, get, post};
use rocket_okapi::openapi;
use sqlx::PgPool;

/// Get the authenticated user's onboarding status and current step
#[openapi(tag = "Onboarding")]
#[get("/status")]
pub async fn get_status(pool: &State<PgPool>, current_user: CurrentUser) -> Result<Json<OnboardingStatusResponse>, AppError> {
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

    // 2. Check if a period schedule exists for the user
    let has_period: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM period_schedule WHERE user_id = $1)")
        .bind(current_user.id)
        .fetch_one(db)
        .await
        .map_err(AppError::from)?;

    if !has_period {
        return Ok(Json(OnboardingStatusResponse {
            status: OnboardingStatus::InProgress,
            current_step: Some(OnboardingStep::Period),
        }));
    }

    // 3. Check if any non-archived accounts exist for the user
    let has_accounts: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM account WHERE user_id = $1 AND is_archived = FALSE)")
        .bind(current_user.id)
        .fetch_one(db)
        .await
        .map_err(AppError::from)?;

    if !has_accounts {
        return Ok(Json(OnboardingStatusResponse {
            status: OnboardingStatus::InProgress,
            current_step: Some(OnboardingStep::Accounts),
        }));
    }

    // 4. Check if at least 1 Incoming AND 1 Outgoing non-archived non-system category exists
    let has_incoming: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM category WHERE user_id = $1 AND is_archived = FALSE AND is_system = FALSE AND category_type = 'Incoming'::category_type)",
    )
    .bind(current_user.id)
    .fetch_one(db)
    .await
    .map_err(AppError::from)?;

    let has_outgoing: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM category WHERE user_id = $1 AND is_archived = FALSE AND is_system = FALSE AND category_type = 'Outgoing'::category_type)",
    )
    .bind(current_user.id)
    .fetch_one(db)
    .await
    .map_err(AppError::from)?;

    if !has_incoming || !has_outgoing {
        return Ok(Json(OnboardingStatusResponse {
            status: OnboardingStatus::InProgress,
            current_step: Some(OnboardingStep::Categories),
        }));
    }

    // 5. All steps done — show summary
    Ok(Json(OnboardingStatusResponse {
        status: OnboardingStatus::InProgress,
        current_step: Some(OnboardingStep::Summary),
    }))
}

/// Mark onboarding as completed for the authenticated user
#[openapi(tag = "Onboarding")]
#[post("/complete")]
pub async fn post_complete(pool: &State<PgPool>, current_user: CurrentUser) -> Result<Status, AppError> {
    let db = pool.inner();

    sqlx::query("UPDATE users SET onboarding_status = 'completed' WHERE id = $1")
        .bind(current_user.id)
        .execute(db)
        .await
        .map_err(AppError::from)?;

    Ok(Status::NoContent)
}

pub fn routes() -> (Vec<rocket::Route>, okapi::openapi3::OpenApi) {
    rocket_okapi::openapi_get_routes_spec![get_status, post_complete]
}
