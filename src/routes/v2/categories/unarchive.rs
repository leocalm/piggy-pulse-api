use rocket::State;
use rocket::http::Status;
use rocket::post;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::service::category::CategoryService;

#[post("/<id>/unarchive")]
pub async fn unarchive_category(pool: &State<PgPool>, user: CurrentUser, id: &str) -> Result<Status, AppError> {
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid category id", e))?;
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = CategoryService::new(&repo);
    service.unarchive_category(&uuid, &user.id).await?;
    Ok(Status::NoContent)
}
