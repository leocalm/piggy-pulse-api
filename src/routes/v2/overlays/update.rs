use rocket::State;
use rocket::put;
use rocket::serde::json::Json;
use sqlx::PgPool;
use uuid::Uuid;
use validator::Validate;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::overlay::{OverlayResponse, UpdateOverlayRequest};
use crate::error::app_error::AppError;
use crate::service::overlay::OverlayService;

#[put("/<id>", data = "<payload>")]
pub async fn update_overlay(pool: &State<PgPool>, user: CurrentUser, id: &str, payload: Json<UpdateOverlayRequest>) -> Result<Json<OverlayResponse>, AppError> {
    payload.validate()?;

    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid overlay id", e))?;
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = OverlayService::new(&repo);
    let response = service.update_overlay(&uuid, &payload, &user.id).await?;
    Ok(Json(response))
}
