use rocket::State;
use rocket::get;
use rocket::serde::json::Json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::vendors::VendorStatsResponse;
use crate::error::app_error::AppError;
use crate::service::vendor::VendorService;

#[get("/stats?<period_id>")]
pub async fn get_vendor_stats(pool: &State<PgPool>, user: CurrentUser, period_id: &str) -> Result<Json<VendorStatsResponse>, AppError> {
    let pid = Uuid::parse_str(period_id).map_err(|e| AppError::uuid("Invalid period id", e))?;

    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = VendorService::new(&repo);
    let response = service.get_stats(&pid, &user.id).await?;
    Ok(Json(response))
}
