use rocket::State;
use rocket::post;
use rocket::serde::json::Json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::categories::EncryptedTargetResponse;
use crate::error::app_error::AppError;
use crate::service::category::CategoryService;

#[post("/<id>/exclude")]
pub async fn exclude_target(pool: &State<PgPool>, user: CurrentUser, id: &str) -> Result<Json<EncryptedTargetResponse>, AppError> {
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid target id", e))?;
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = CategoryService::new(&repo);
    Ok(Json(service.toggle_target_excluded(&uuid, &user.id).await?))
}
