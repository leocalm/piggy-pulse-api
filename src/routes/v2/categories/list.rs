use rocket::State;
use rocket::get;
use rocket::serde::json::Json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::categories::CategoryManagementListResponse;
use crate::error::app_error::AppError;
use crate::service::category::CategoryService;

#[get("/?<cursor>&<limit>")]
pub async fn list_categories(
    pool: &State<PgPool>,
    user: CurrentUser,
    cursor: Option<String>,
    limit: Option<u32>,
) -> Result<Json<CategoryManagementListResponse>, AppError> {
    let cursor_uuid = match cursor {
        Some(ref s) if !s.is_empty() && s != "null" => Some(Uuid::parse_str(s).map_err(|e| AppError::uuid("Invalid cursor", e))?),
        _ => None,
    };
    let effective_limit = limit.unwrap_or(20).min(100) as i64;

    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = CategoryService::new(&repo);

    let response = service.list_categories_v2(cursor_uuid, effective_limit, &user.id).await?;
    Ok(Json(response))
}
