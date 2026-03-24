use rocket::State;
use rocket::get;
use rocket::serde::json::Json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::dashboard::UncategorizedResponse;
use crate::error::app_error::AppError;
use crate::service::dashboard::DashboardService;

#[get("/uncategorized?<periodId>")]
pub async fn get_uncategorized(
    pool: &State<PgPool>,
    user: CurrentUser,
    #[allow(non_snake_case)] periodId: Option<String>,
) -> Result<Json<UncategorizedResponse>, AppError> {
    let period_uuid = match periodId {
        Some(ref s) => Uuid::parse_str(s).map_err(|e| AppError::uuid("Invalid period id", e))?,
        None => return Err(AppError::BadRequest("periodId is required".to_string())),
    };

    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = DashboardService::new(&repo);

    let response = service.get_uncategorized(&period_uuid, &user.id).await?;
    Ok(Json(response))
}
