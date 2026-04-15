use rocket::State;
use rocket::http::Status;
use rocket::post;
use rocket::serde::json::Json;
use sqlx::PgPool;
use validator::Validate;

use crate::auth::CurrentUser;
use crate::crypto::Dek;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::vendors::{CreateVendorRequest, EncryptedVendorResponse};
use crate::error::app_error::AppError;
use crate::service::vendor::VendorService;

#[post("/", data = "<payload>")]
pub async fn create_vendor(
    pool: &State<PgPool>,
    user: CurrentUser,
    dek: Dek,
    payload: Json<CreateVendorRequest>,
) -> Result<(Status, Json<EncryptedVendorResponse>), AppError> {
    payload.validate()?;
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = VendorService::new(&repo);
    Ok((Status::Created, Json(service.create_vendor(&payload, &user.id, &dek).await?)))
}
