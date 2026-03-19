use rocket::State;
use rocket::put;
use rocket::serde::json::Json;
use sqlx::PgPool;
use uuid::Uuid;
use validator::Validate;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::categories::{CategoryResponse, UpdateCategoryRequest};
use crate::error::app_error::AppError;
use crate::service::category::CategoryService;

#[put("/<id>", data = "<payload>")]
pub async fn update_category(
    pool: &State<PgPool>,
    user: CurrentUser,
    id: &str,
    payload: Json<UpdateCategoryRequest>,
) -> Result<Json<CategoryResponse>, AppError> {
    payload.validate()?;

    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid category id", e))?;
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = CategoryService::new(&repo);

    let response = service.update_category(&uuid, &payload, &user.id).await?;
    Ok(Json(response))
}
