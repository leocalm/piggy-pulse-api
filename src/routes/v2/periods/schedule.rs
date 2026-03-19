use rocket::State;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{delete, get, post, put};
use sqlx::PgPool;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::period::{CreatePeriodScheduleRequest, PeriodScheduleResponse, UpdatePeriodScheduleRequest};
use crate::error::app_error::AppError;
use crate::service::period::PeriodService;

#[get("/schedule")]
pub async fn get_schedule(pool: &State<PgPool>, user: CurrentUser) -> Result<Json<PeriodScheduleResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = PeriodService::new(&repo);
    let response = service.get_schedule(&user.id).await?;
    Ok(Json(response))
}

#[post("/schedule", data = "<payload>")]
pub async fn create_schedule(
    pool: &State<PgPool>,
    user: CurrentUser,
    payload: Json<CreatePeriodScheduleRequest>,
) -> Result<(Status, Json<PeriodScheduleResponse>), AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = PeriodService::new(&repo);
    let response = service.create_schedule(&payload, &user.id).await?;
    Ok((Status::Created, Json(response)))
}

#[put("/schedule", data = "<payload>")]
pub async fn update_schedule(
    pool: &State<PgPool>,
    user: CurrentUser,
    payload: Json<UpdatePeriodScheduleRequest>,
) -> Result<Json<PeriodScheduleResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = PeriodService::new(&repo);
    let response = service.update_schedule(&payload, &user.id).await?;
    Ok(Json(response))
}

#[delete("/schedule")]
pub async fn delete_schedule(pool: &State<PgPool>, user: CurrentUser) -> Result<Status, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = PeriodService::new(&repo);
    service.delete_schedule(&user.id).await?;
    Ok(Status::NoContent)
}
