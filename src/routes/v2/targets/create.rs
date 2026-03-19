use rocket::State;
use rocket::http::Status;
use rocket::post;
use rocket::serde::json::Json;
use sqlx::PgPool;
use validator::Validate;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::categories::{CreateTargetRequest, TargetItem};
use crate::error::app_error::AppError;
use crate::service::category::CategoryService;

#[post("/", data = "<payload>")]
pub async fn create_target(pool: &State<PgPool>, user: CurrentUser, payload: Json<CreateTargetRequest>) -> Result<(Status, Json<TargetItem>), AppError> {
    payload.validate()?;

    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = CategoryService::new(&repo);

    let response = service.create_target(&payload, &user.id).await?;
    Ok((Status::Created, Json(response)))
}
