use rocket::State;
use rocket::get;
use rocket::serde::json::Json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::subscriptions::{SubscriptionListResponse, SubscriptionStatus};
use crate::error::app_error::AppError;
use crate::service::subscription::SubscriptionService;

#[get("/?<status>&<categoryId>")]
pub async fn list_subscriptions(
    pool: &State<PgPool>,
    user: CurrentUser,
    status: Option<String>,
    #[allow(non_snake_case)] categoryId: Option<String>,
) -> Result<Json<SubscriptionListResponse>, AppError> {
    let status_filter = match status.as_deref() {
        Some("active") => Some(SubscriptionStatus::Active),
        Some("cancelled") => Some(SubscriptionStatus::Cancelled),
        Some("paused") => Some(SubscriptionStatus::Paused),
        Some(s) => return Err(AppError::BadRequest(format!("Invalid status '{}'. Must be: active, cancelled, paused", s))),
        None => None,
    };

    let category_uuid = match categoryId {
        Some(ref id) => Some(Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid categoryId", e))?),
        None => None,
    };

    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = SubscriptionService::new(&repo);
    let response = service.list(&user.id, status_filter, category_uuid).await?;
    Ok(Json(response))
}
