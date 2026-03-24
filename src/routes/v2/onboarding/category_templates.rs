use rocket::State;
use rocket::get;
use rocket::serde::json::Json;
use sqlx::PgPool;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::misc::CategoryTemplateResponse;
use crate::error::app_error::AppError;
use crate::service::onboarding::OnboardingService;

#[get("/category-templates")]
pub async fn list_category_templates(pool: &State<PgPool>, user: CurrentUser) -> Result<Json<Vec<CategoryTemplateResponse>>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = OnboardingService::new(&repo);
    // user guard ensures authentication; templates are static but endpoint is auth-gated
    let _ = user;
    Ok(Json(service.list_templates()))
}
