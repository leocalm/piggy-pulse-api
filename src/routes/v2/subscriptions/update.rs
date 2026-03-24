use rocket::State;
use rocket::put;
use rocket::serde::json::Json;
use sqlx::PgPool;
use uuid::Uuid;
use validator::Validate;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::subscriptions::{SubscriptionResponse, UpdateSubscriptionRequest};
use crate::error::app_error::AppError;
use crate::service::subscription::SubscriptionService;

#[put("/<id>", data = "<payload>")]
pub async fn update_subscription(
    pool: &State<PgPool>,
    user: CurrentUser,
    id: &str,
    payload: Json<UpdateSubscriptionRequest>,
) -> Result<Json<SubscriptionResponse>, AppError> {
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid subscription id", e))?;
    payload.validate()?;

    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = SubscriptionService::new(&repo);
    let response = service.update(&uuid, &payload, &user.id).await?;
    Ok(Json(response))
}
