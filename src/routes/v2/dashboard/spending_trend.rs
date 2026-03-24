use rocket::State;
use rocket::get;
use rocket::serde::json::Json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::dashboard::SpendingTrendResponse;
use crate::error::app_error::AppError;
use crate::service::dashboard::DashboardService;

#[get("/spending-trend?<periodId>&<limit>")]
pub async fn get_spending_trend(
    pool: &State<PgPool>,
    user: CurrentUser,
    #[allow(non_snake_case)] periodId: Option<String>,
    limit: Option<i64>,
) -> Result<Json<SpendingTrendResponse>, AppError> {
    let period_uuid = match periodId {
        Some(ref s) => Uuid::parse_str(s).map_err(|e| AppError::uuid("Invalid period id", e))?,
        None => return Err(AppError::BadRequest("periodId is required".to_string())),
    };

    let limit = limit.unwrap_or(12).clamp(1, 50);

    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = DashboardService::new(&repo);

    let response = service.get_spending_trend(&period_uuid, &user.id, limit).await?;
    Ok(Json(response))
}
