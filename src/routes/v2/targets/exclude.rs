use rocket::State;
use rocket::post;
use rocket::serde::json::Json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::categories::TargetItem;
use crate::error::app_error::AppError;
use crate::service::category::CategoryService;

#[post("/<id>/exclude")]
pub async fn exclude_target(pool: &State<PgPool>, user: CurrentUser, id: &str) -> Result<Json<TargetItem>, AppError> {
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid target id", e))?;
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = CategoryService::new(&repo);

    let response = service.exclude_target(&uuid, &user.id).await?;
    Ok(Json(response))
}
