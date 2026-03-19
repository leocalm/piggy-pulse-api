use rocket::State;
use rocket::put;
use rocket::serde::json::Json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::period::{PeriodResponse, UpdatePeriodRequest};
use crate::error::app_error::AppError;
use crate::service::period::PeriodService;

#[put("/<id>", data = "<payload>")]
pub async fn update_period(pool: &State<PgPool>, user: CurrentUser, id: &str, payload: Json<UpdatePeriodRequest>) -> Result<Json<PeriodResponse>, AppError> {
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid period id", e))?;
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = PeriodService::new(&repo);
    let response = service.update_period(&uuid, &payload, &user.id).await?;
    Ok(Json(response))
}
