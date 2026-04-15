use rocket::State;
use rocket::post;
use rocket::serde::json::Json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::subscriptions::{CancelSubscriptionRequest, EncryptedSubscriptionResponse};
use crate::error::app_error::AppError;
use crate::service::subscription::SubscriptionService;

#[post("/<id>/cancel", data = "<payload>")]
pub async fn cancel_subscription(
    pool: &State<PgPool>,
    user: CurrentUser,
    id: &str,
    payload: Json<CancelSubscriptionRequest>,
) -> Result<Json<EncryptedSubscriptionResponse>, AppError> {
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid subscription id", e))?;
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = SubscriptionService::new(&repo);
    let cancellation_date = payload.cancellation_date.as_ref().map(|d| d.0);
    Ok(Json(service.cancel(&uuid, &user.id, cancellation_date.as_ref()).await?))
}
