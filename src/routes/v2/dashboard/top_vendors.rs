use rocket::State;
use rocket::get;
use rocket::serde::json::Json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::dashboard::TopVendorsResponse;
use crate::error::app_error::AppError;
use crate::service::dashboard::DashboardService;

#[get("/top-vendors?<periodId>&<limit>")]
pub async fn get_top_vendors(
    pool: &State<PgPool>,
    user: CurrentUser,
    #[allow(non_snake_case)] periodId: Option<String>,
    limit: Option<i64>,
) -> Result<Json<TopVendorsResponse>, AppError> {
    let period_uuid = match periodId {
        Some(ref s) => Uuid::parse_str(s).map_err(|e| AppError::uuid("Invalid period id", e))?,
        None => return Err(AppError::BadRequest("periodId is required".to_string())),
    };

    let limit = limit.unwrap_or(10).clamp(1, 50);

    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = DashboardService::new(&repo);

    let response = service.get_top_vendors(&period_uuid, &user.id, limit).await?;
    Ok(Json(response))
}
