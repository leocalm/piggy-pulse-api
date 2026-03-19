use rocket::State;
use rocket::get;
use rocket::serde::json::Json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::overlay::OverlayTransactionListResponse;
use crate::error::app_error::AppError;
use crate::service::overlay::OverlayService;

#[get("/<id>/transactions")]
pub async fn list_overlay_transactions(pool: &State<PgPool>, user: CurrentUser, id: &str) -> Result<Json<OverlayTransactionListResponse>, AppError> {
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid overlay id", e))?;

    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = OverlayService::new(&repo);
    let response = service.list_overlay_transactions(&uuid, &user.id).await?;
    Ok(Json(response))
}
