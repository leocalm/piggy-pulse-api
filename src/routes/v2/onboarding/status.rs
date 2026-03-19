use rocket::State;
use rocket::get;
use rocket::serde::json::Json;
use sqlx::PgPool;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::misc::OnboardingStatusResponse;
use crate::error::app_error::AppError;
use crate::service::onboarding::OnboardingService;

#[get("/status")]
pub async fn get_onboarding_status(pool: &State<PgPool>, user: CurrentUser) -> Result<Json<OnboardingStatusResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = OnboardingService::new(&repo);
    let response = service.get_status(&user.id).await?;
    Ok(Json(response))
}
