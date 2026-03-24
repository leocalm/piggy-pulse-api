use rocket::State;
use rocket::post;
use rocket::serde::json::Json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::subscriptions::SubscriptionResponse;
use crate::error::app_error::AppError;
use crate::service::subscription::SubscriptionService;

#[post("/<id>/cancel")]
pub async fn cancel_subscription(pool: &State<PgPool>, user: CurrentUser, id: &str) -> Result<Json<SubscriptionResponse>, AppError> {
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid subscription id", e))?;

    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = SubscriptionService::new(&repo);
    let response = service.cancel(&uuid, &user.id).await?;
    Ok(Json(response))
}
