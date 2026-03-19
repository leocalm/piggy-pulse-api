use rocket::State;
use rocket::get;
use rocket::serde::json::Json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::categories::CategoryOverviewResponse;
use crate::error::app_error::AppError;
use crate::service::category::CategoryService;

#[get("/overview?<periodId>")]
pub async fn category_overview(
    pool: &State<PgPool>,
    user: CurrentUser,
    #[allow(non_snake_case)] periodId: Option<String>,
) -> Result<Json<CategoryOverviewResponse>, AppError> {
    let period_uuid = match periodId {
        Some(ref s) if !s.is_empty() => Uuid::parse_str(s).map_err(|e| AppError::uuid("Invalid periodId", e))?,
        _ => return Err(AppError::BadRequest("periodId is required".to_string())),
    };

    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = CategoryService::new(&repo);

    let response = service.get_category_overview(&period_uuid, &user.id).await?;
    Ok(Json(response))
}
