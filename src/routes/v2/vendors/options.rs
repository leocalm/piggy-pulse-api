use rocket::State;
use rocket::get;
use rocket::serde::json::Json;
use sqlx::PgPool;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::vendors::VendorOptionListResponse;
use crate::error::app_error::AppError;
use crate::service::vendor::VendorService;

#[get("/options")]
pub async fn list_vendor_options(pool: &State<PgPool>, user: CurrentUser) -> Result<Json<VendorOptionListResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = VendorService::new(&repo);

    let response = service.list_vendor_options(&user.id).await?;
    Ok(Json(response))
}
