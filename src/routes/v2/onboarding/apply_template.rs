use rocket::State;
use rocket::http::Status;
use rocket::post;
use rocket::serde::json::Json;
use sqlx::PgPool;

use crate::auth::CurrentUser;
use crate::crypto::Dek;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::categories::EncryptedCategoryResponse;
use crate::dto::misc::ApplyTemplateRequest;
use crate::error::app_error::AppError;
use crate::service::onboarding::OnboardingService;

#[post("/apply-template", data = "<payload>")]
pub async fn apply_template(
    pool: &State<PgPool>,
    user: CurrentUser,
    dek: Dek,
    payload: Json<ApplyTemplateRequest>,
) -> Result<(Status, Json<Vec<EncryptedCategoryResponse>>), AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = OnboardingService::new(&repo);
    let categories = service.apply_template(&payload, &user.id, &dek).await?;
    Ok((Status::Created, Json(categories)))
}
