use rocket::State;
use rocket::post;
use rocket::serde::json::Json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::vendors::VendorResponse;
use crate::error::app_error::AppError;
use crate::service::vendor::VendorService;

#[post("/<id>/archive")]
pub async fn archive_vendor(pool: &State<PgPool>, user: CurrentUser, id: &str) -> Result<Json<VendorResponse>, AppError> {
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid vendor id", e))?;
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = VendorService::new(&repo);

    let response = service.archive_vendor(&uuid, &user.id).await?;
    Ok(Json(response))
}
