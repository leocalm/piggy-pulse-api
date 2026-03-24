use rocket::State;
use rocket::get;
use rocket::serde::json::Json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::categories::CategoryTrendResponse;
use crate::error::app_error::AppError;
use crate::service::category::CategoryService;

#[get("/<id>/trend?<limit>")]
pub async fn get_category_trend(pool: &State<PgPool>, user: CurrentUser, id: &str, limit: Option<i64>) -> Result<Json<CategoryTrendResponse>, AppError> {
    let category_uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid category id", e))?;
    let effective_limit = limit.unwrap_or(6).clamp(1, 24);

    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = CategoryService::new(&repo);
    let response = service.get_category_trend(&category_uuid, effective_limit, &user.id).await?;
    Ok(Json(response))
}
