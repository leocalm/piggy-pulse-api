use rocket::State;
use rocket::http::Status;
use rocket::post;
use rocket::serde::json::Json;
use sqlx::PgPool;
use validator::Validate;

use crate::auth::CurrentUser;
use crate::crypto::Dek;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::subscriptions::{CreateSubscriptionRequest, EncryptedSubscriptionResponse};
use crate::error::app_error::AppError;
use crate::service::subscription::SubscriptionService;

#[post("/", data = "<payload>")]
pub async fn create_subscription(
    pool: &State<PgPool>,
    user: CurrentUser,
    dek: Dek,
    payload: Json<CreateSubscriptionRequest>,
) -> Result<(Status, Json<EncryptedSubscriptionResponse>), AppError> {
    payload.validate()?;
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = SubscriptionService::new(&repo);
    Ok((Status::Created, Json(service.create(&payload, &user.id, &dek).await?)))
}
