use rocket::State;
use rocket::http::Status;
use rocket::post;
use rocket::serde::json::Json;
use sqlx::PgPool;
use validator::Validate;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::vendors::{CreateVendorRequest, VendorResponse};
use crate::error::app_error::AppError;
use crate::service::vendor::VendorService;

#[post("/", data = "<payload>")]
pub async fn create_vendor(pool: &State<PgPool>, user: CurrentUser, payload: Json<CreateVendorRequest>) -> Result<(Status, Json<VendorResponse>), AppError> {
    payload.validate()?;

    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = VendorService::new(&repo);

    let response = service.create_vendor(&payload, &user.id).await?;
    Ok((Status::Created, Json(response)))
}
