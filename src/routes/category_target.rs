use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::middleware::rate_limit::RateLimit;
use crate::models::category_target::{BatchUpsertTargetsRequest, CategoryTargetsResponse};
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{State, get, post};
use rocket_okapi::openapi;
use sqlx::PgPool;
use uuid::Uuid;
use validator::Validate;

/// Get all category targets for a given budget period
#[openapi(tag = "Category Targets")]
#[get("/?<period_id>")]
pub async fn get_category_targets(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
    period_id: String,
) -> Result<Json<CategoryTargetsResponse>, AppError> {
    let uuid = Uuid::parse_str(&period_id).map_err(|e| AppError::uuid("Invalid period_id", e))?;

    let repo = PostgresRepository { pool: pool.inner().clone() };
    let response = repo.get_category_targets(&uuid, &current_user.id).await?;
    Ok(Json(response))
}

/// Batch upsert category targets for a period
#[openapi(tag = "Category Targets")]
#[post("/", data = "<payload>")]
pub async fn save_category_targets(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
    payload: Json<BatchUpsertTargetsRequest>,
) -> Result<Status, AppError> {
    payload.validate()?;

    let repo = PostgresRepository { pool: pool.inner().clone() };
    repo.batch_upsert_targets(&payload.targets, &current_user.id).await?;
    Ok(Status::Ok)
}

/// Exclude a category from target tracking
#[openapi(tag = "Category Targets")]
#[post("/<id>/exclude")]
pub async fn exclude_category(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
    id: &str,
) -> Result<Status, AppError> {
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid category id", e))?;

    let repo = PostgresRepository { pool: pool.inner().clone() };
    repo.exclude_category_from_targets(&uuid, &current_user.id).await?;
    Ok(Status::Ok)
}

/// Re-include a category in target tracking
#[openapi(tag = "Category Targets")]
#[post("/<id>/include")]
pub async fn include_category(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
    id: &str,
) -> Result<Status, AppError> {
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid category id", e))?;

    let repo = PostgresRepository { pool: pool.inner().clone() };
    repo.include_category_in_targets(&uuid, &current_user.id).await?;
    Ok(Status::Ok)
}

pub fn routes() -> (Vec<rocket::Route>, okapi::openapi3::OpenApi) {
    rocket_okapi::openapi_get_routes_spec![
        get_category_targets,
        save_category_targets,
        exclude_category,
        include_category
    ]
}
