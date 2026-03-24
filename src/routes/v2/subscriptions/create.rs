use rocket::State;
use rocket::http::Status;
use rocket::post;
use rocket::serde::json::Json;
use sqlx::PgPool;
use validator::Validate;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::subscriptions::{CreateSubscriptionRequest, SubscriptionResponse};
use crate::error::app_error::AppError;
use crate::service::subscription::SubscriptionService;

#[post("/", data = "<payload>")]
pub async fn create_subscription(
    pool: &State<PgPool>,
    user: CurrentUser,
    payload: Json<CreateSubscriptionRequest>,
) -> Result<(Status, Json<SubscriptionResponse>), AppError> {
    payload.validate()?;

    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = SubscriptionService::new(&repo);
    let response = service.create(&payload, &user.id).await?;
    Ok((Status::Created, Json(response)))
}
