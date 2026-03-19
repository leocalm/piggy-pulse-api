use rocket::State;
use rocket::http::Status;
use rocket::post;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::service::overlay::OverlayService;

#[post("/<id>/transactions/<tx_id>/include")]
pub async fn include_overlay_transaction(pool: &State<PgPool>, user: CurrentUser, id: &str, tx_id: &str) -> Result<Status, AppError> {
    let overlay_uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid overlay id", e))?;
    let tx_uuid = Uuid::parse_str(tx_id).map_err(|e| AppError::uuid("Invalid transaction id", e))?;

    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = OverlayService::new(&repo);
    service.include_transaction(&overlay_uuid, &tx_uuid, &user.id).await?;
    Ok(Status::NoContent)
}
