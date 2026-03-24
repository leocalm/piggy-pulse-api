use rocket::State;
use rocket::http::Status;
use rocket::post;
use rocket::serde::json::Json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::vendors::MergeVendorRequest;
use crate::error::app_error::AppError;
use crate::service::vendor::VendorService;

#[post("/<id>/merge", data = "<payload>")]
pub async fn merge_vendor(pool: &State<PgPool>, user: CurrentUser, id: &str, payload: Json<MergeVendorRequest>) -> Result<Status, AppError> {
    let source_uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid vendor id", e))?;

    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = VendorService::new(&repo);
    service.merge_vendor(&source_uuid, &payload, &user.id).await?;
    Ok(Status::NoContent)
}
