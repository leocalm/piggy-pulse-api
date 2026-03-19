use rocket::State;
use rocket::http::Status;
use rocket::post;
use sqlx::PgPool;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::service::onboarding::OnboardingService;

#[post("/complete")]
pub async fn complete_onboarding(pool: &State<PgPool>, user: CurrentUser) -> Result<Status, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = OnboardingService::new(&repo);
    service.complete(&user.id).await?;
    Ok(Status::NoContent)
}
