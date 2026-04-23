use rocket::State;
use rocket::get;
use rocket::serde::json::Json;
use sqlx::PgPool;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::vendors::VendorListResponse;
use crate::error::app_error::AppError;
use crate::service::vendor::VendorService;

#[get("/")]
pub async fn list_vendors(pool: &State<PgPool>, user: CurrentUser) -> Result<Json<VendorListResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = VendorService::new(&repo);
    Ok(Json(service.list_vendors(&user.id).await?))
}
