use rocket::State;
use rocket::get;
use rocket::serde::json::Json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::vendors::VendorDetailResponse;
use crate::error::app_error::AppError;
use crate::service::vendor::VendorService;

#[get("/<id>/detail?<periodId>")]
pub async fn get_vendor_detail(
    pool: &State<PgPool>,
    user: CurrentUser,
    id: &str,
    #[allow(non_snake_case)] periodId: Option<String>,
) -> Result<Json<VendorDetailResponse>, AppError> {
    let vendor_uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid vendor id", e))?;
    let period_uuid = match periodId {
        Some(ref s) => Uuid::parse_str(s).map_err(|e| AppError::uuid("Invalid period id", e))?,
        None => return Err(AppError::BadRequest("periodId is required".to_string())),
    };

    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = VendorService::new(&repo);
    let response = service.get_vendor_detail(&vendor_uuid, &period_uuid, &user.id).await?;
    Ok(Json(response))
}
