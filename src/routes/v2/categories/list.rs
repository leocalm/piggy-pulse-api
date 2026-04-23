use rocket::State;
use rocket::get;
use rocket::serde::json::Json;
use sqlx::PgPool;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::categories::CategoryListResponse;
use crate::error::app_error::AppError;
use crate::service::category::CategoryService;

#[get("/")]
pub async fn list_categories(pool: &State<PgPool>, user: CurrentUser) -> Result<Json<CategoryListResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = CategoryService::new(&repo);
    Ok(Json(service.list_categories(&user.id).await?))
}
