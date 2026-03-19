use rocket::State;
use rocket::put;
use rocket::serde::json::Json;
use sqlx::PgPool;
use uuid::Uuid;
use validator::Validate;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::vendors::{UpdateVendorRequest, VendorResponse};
use crate::error::app_error::AppError;
use crate::service::vendor::VendorService;

#[put("/<id>", data = "<payload>")]
pub async fn update_vendor(pool: &State<PgPool>, user: CurrentUser, id: &str, payload: Json<UpdateVendorRequest>) -> Result<Json<VendorResponse>, AppError> {
    payload.validate()?;

    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid vendor id", e))?;
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = VendorService::new(&repo);

    let response = service.update_vendor(&uuid, &payload, &user.id).await?;
    Ok(Json(response))
}
