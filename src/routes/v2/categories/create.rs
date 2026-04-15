use rocket::State;
use rocket::http::Status;
use rocket::post;
use rocket::serde::json::Json;
use sqlx::PgPool;
use validator::Validate;

use crate::auth::CurrentUser;
use crate::crypto::Dek;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::categories::{CreateCategoryRequest, EncryptedCategoryResponse};
use crate::error::app_error::AppError;
use crate::service::category::CategoryService;

#[post("/", data = "<payload>")]
pub async fn create_category(
    pool: &State<PgPool>,
    user: CurrentUser,
    dek: Dek,
    payload: Json<CreateCategoryRequest>,
) -> Result<(Status, Json<EncryptedCategoryResponse>), AppError> {
    payload.validate()?;
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = CategoryService::new(&repo);
    let response = service.create_category(&payload, &user.id, &dek).await?;
    Ok((Status::Created, Json(response)))
}
