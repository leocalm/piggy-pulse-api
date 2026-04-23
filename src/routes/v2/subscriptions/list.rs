use rocket::State;
use rocket::get;
use rocket::serde::json::Json;
use sqlx::PgPool;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::subscriptions::SubscriptionListResponse;
use crate::error::app_error::AppError;
use crate::service::subscription::SubscriptionService;

#[get("/")]
pub async fn list_subscriptions(pool: &State<PgPool>, user: CurrentUser) -> Result<Json<SubscriptionListResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = SubscriptionService::new(&repo);
    Ok(Json(service.list(&user.id).await?))
}
