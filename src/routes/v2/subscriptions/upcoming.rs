use rocket::State;
use rocket::get;
use rocket::serde::json::Json;
use sqlx::PgPool;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::subscriptions::UpcomingChargesResponse;
use crate::error::app_error::AppError;
use crate::service::subscription::SubscriptionService;

#[get("/upcoming?<limit>")]
pub async fn get_upcoming(pool: &State<PgPool>, user: CurrentUser, limit: Option<i64>) -> Result<Json<UpcomingChargesResponse>, AppError> {
    let limit = limit.unwrap_or(10).clamp(1, 50);

    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = SubscriptionService::new(&repo);
    let response = service.upcoming(&user.id, limit).await?;
    Ok(Json(response))
}
