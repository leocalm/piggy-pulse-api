use rocket::State;
use rocket::get;
use rocket::serde::json::Json;
use sqlx::PgPool;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::categories::CategoryOptionListResponse;
use crate::error::app_error::AppError;
use crate::service::category::CategoryService;

#[get("/options")]
pub async fn list_category_options(pool: &State<PgPool>, user: CurrentUser) -> Result<Json<CategoryOptionListResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = CategoryService::new(&repo);

    let response = service.list_category_options(&user.id).await?;
    Ok(Json(response))
}
