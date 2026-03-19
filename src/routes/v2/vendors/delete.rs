use rocket::State;
use rocket::delete;
use rocket::http::Status;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::service::vendor::VendorService;

#[delete("/<id>")]
pub async fn delete_vendor(pool: &State<PgPool>, user: CurrentUser, id: &str) -> Result<Status, AppError> {
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid vendor id", e))?;
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = VendorService::new(&repo);

    service.delete_vendor(&uuid, &user.id).await?;
    Ok(Status::NoContent)
}
