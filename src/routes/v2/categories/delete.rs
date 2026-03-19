use rocket::State;
use rocket::delete;
use rocket::http::Status;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;

#[delete("/<id>")]
pub async fn delete_category(pool: &State<PgPool>, user: CurrentUser, id: &str) -> Result<Status, AppError> {
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid category id", e))?;
    let repo = PostgresRepository { pool: pool.inner().clone() };

    repo.get_category_by_id(&uuid, &user.id)
        .await?
        .ok_or_else(|| AppError::NotFound("Category not found".to_string()))?;

    repo.delete_category(&uuid, &user.id).await?;
    Ok(Status::NoContent)
}
