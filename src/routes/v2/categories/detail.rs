use rocket::State;
use rocket::get;
use rocket::serde::json::Json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::categories::CategoryDetailResponse;
use crate::error::app_error::AppError;
use crate::service::category::CategoryService;

#[allow(non_snake_case)]
#[get("/<id>/detail?<periodId>")]
pub async fn get_category_detail(
    pool: &State<PgPool>,
    user: CurrentUser,
    id: &str,
    periodId: Option<String>,
) -> Result<Json<CategoryDetailResponse>, AppError> {
    let category_uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid category id", e))?;
    let period_str = periodId.ok_or_else(|| AppError::BadRequest("periodId is required".to_string()))?;
    let period_uuid = Uuid::parse_str(&period_str).map_err(|e| AppError::uuid("Invalid period id", e))?;

    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = CategoryService::new(&repo);
    let response = service.get_category_detail(&category_uuid, &period_uuid, &user.id).await?;
    Ok(Json(response))
}
