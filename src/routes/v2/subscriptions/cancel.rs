use rocket::State;
use rocket::post;
use rocket::serde::json::Json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::subscriptions::{CancelSubscriptionRequest, SubscriptionResponse};
use crate::error::app_error::AppError;
use crate::service::subscription::SubscriptionService;

#[post("/<id>/cancel", data = "<body>")]
pub async fn cancel_subscription(
    pool: &State<PgPool>,
    user: CurrentUser,
    id: &str,
    body: Option<Json<CancelSubscriptionRequest>>,
) -> Result<Json<SubscriptionResponse>, AppError> {
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid subscription id", e))?;
    let cancellation_date = body.and_then(|b| b.into_inner().cancellation_date).map(|d| d.0);

    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = SubscriptionService::new(&repo);
    let response = service.cancel(&uuid, &user.id, cancellation_date.as_ref()).await?;
    Ok(Json(response))
}
