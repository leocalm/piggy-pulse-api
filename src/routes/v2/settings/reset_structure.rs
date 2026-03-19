use rocket::State;
use rocket::http::Status;
use rocket::post;
use rocket::serde::json::Json;
use sqlx::PgPool;
use validator::Validate;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::settings::ResetStructureRequest;
use crate::error::app_error::AppError;
use crate::service::settings::SettingsService;

#[post("/reset-structure", data = "<payload>")]
pub async fn reset_structure(pool: &State<PgPool>, user: CurrentUser, payload: Json<ResetStructureRequest>) -> Result<Status, AppError> {
    payload.validate()?;
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = SettingsService::new(&repo);
    service.reset_structure(&user.id, &payload.password).await?;
    Ok(Status::NoContent)
}
