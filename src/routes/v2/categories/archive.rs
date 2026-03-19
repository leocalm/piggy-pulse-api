use rocket::State;
use rocket::post;
use rocket::serde::json::Json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::categories::CategoryResponse;
use crate::error::app_error::AppError;
use crate::service::category::CategoryService;

#[post("/<id>/archive")]
pub async fn archive_category(pool: &State<PgPool>, user: CurrentUser, id: &str) -> Result<Json<CategoryResponse>, AppError> {
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid category id", e))?;
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = CategoryService::new(&repo);

    let response = service.archive_category(&uuid, &user.id).await?;
    Ok(Json(response))
}
