use rocket::State;
use rocket::serde::json::Json;
use rocket::{get, put};
use sqlx::PgPool;
use validator::Validate;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::settings::{PreferencesResponse, UpdatePreferencesRequest};
use crate::error::app_error::AppError;
use crate::service::settings::SettingsService;

#[get("/preferences")]
pub async fn get_preferences(pool: &State<PgPool>, user: CurrentUser) -> Result<Json<PreferencesResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = SettingsService::new(&repo);
    let prefs = service.get_preferences(&user.id).await?;
    Ok(Json(prefs))
}

#[put("/preferences", data = "<payload>")]
pub async fn update_preferences(
    pool: &State<PgPool>,
    user: CurrentUser,
    payload: Json<UpdatePreferencesRequest>,
) -> Result<Json<PreferencesResponse>, AppError> {
    payload.validate()?;
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = SettingsService::new(&repo);
    let prefs = service.update_preferences(&user.id, &payload).await?;
    Ok(Json(prefs))
}
