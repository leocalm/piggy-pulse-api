use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::middleware::rate_limit::RateLimit;
use crate::models::settings::{SettingsRequest, SettingsResponse};
use rocket::serde::json::Json;
use rocket::{State, get, put};
use rocket_okapi::openapi;
use sqlx::PgPool;
use validator::Validate;

/// Get current user's settings
#[openapi(tag = "Settings")]
#[get("/")]
pub async fn get_settings(pool: &State<PgPool>, _rate_limit: RateLimit, current_user: CurrentUser) -> Result<Json<SettingsResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let settings = repo.get_settings(&current_user.id).await?;
    Ok(Json(SettingsResponse::from(&settings)))
}

/// Update current user's settings (creates if not exists)
#[openapi(tag = "Settings")]
#[put("/", data = "<payload>")]
pub async fn put_settings(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
    payload: Json<SettingsRequest>,
) -> Result<Json<SettingsResponse>, AppError> {
    payload.validate()?;

    let repo = PostgresRepository { pool: pool.inner().clone() };
    let settings = repo.upsert_settings(&payload, &current_user.id).await?;
    Ok(Json(SettingsResponse::from(&settings)))
}

pub fn routes() -> (Vec<rocket::Route>, okapi::openapi3::OpenApi) {
    rocket_okapi::openapi_get_routes_spec![get_settings, put_settings]
}
