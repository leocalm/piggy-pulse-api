use rocket::State;
use rocket::http::Status;
use rocket::post;
use rocket::serde::json::Json;
use sqlx::PgPool;
use validator::Validate;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::overlay::{CreateOverlayRequest, OverlayResponse};
use crate::error::app_error::AppError;
use crate::service::overlay::OverlayService;

#[post("/", data = "<payload>")]
pub async fn create_overlay(pool: &State<PgPool>, user: CurrentUser, payload: Json<CreateOverlayRequest>) -> Result<(Status, Json<OverlayResponse>), AppError> {
    payload.validate()?;

    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = OverlayService::new(&repo);
    let response = service.create_overlay(&payload, &user.id).await?;
    Ok((Status::Created, Json(response)))
}
