use rocket::State;
use rocket::serde::json::Json;
use rocket::{get, put};
use sqlx::PgPool;
use validator::Validate;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::settings::{ProfileResponse, UpdateProfileRequest};
use crate::error::app_error::AppError;
use crate::service::settings::SettingsService;

#[get("/profile")]
pub async fn get_profile(pool: &State<PgPool>, user: CurrentUser) -> Result<Json<ProfileResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = SettingsService::new(&repo);
    let profile = service.get_profile(&user.id).await?;
    Ok(Json(profile))
}

#[put("/profile", data = "<payload>")]
pub async fn update_profile(pool: &State<PgPool>, user: CurrentUser, payload: Json<UpdateProfileRequest>) -> Result<Json<ProfileResponse>, AppError> {
    payload.validate()?;
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = SettingsService::new(&repo);
    let profile = service.update_profile(&user.id, &payload).await?;
    Ok(Json(profile))
}
