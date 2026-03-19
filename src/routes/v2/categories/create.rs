use rocket::State;
use rocket::http::Status;
use rocket::post;
use rocket::serde::json::Json;
use sqlx::PgPool;
use validator::Validate;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::categories::{CategoryResponse, CreateCategoryRequest};
use crate::error::app_error::AppError;
use crate::models::category::CategoryRequest;

#[post("/", data = "<payload>")]
pub async fn create_category(
    pool: &State<PgPool>,
    user: CurrentUser,
    payload: Json<CreateCategoryRequest>,
) -> Result<(Status, Json<CategoryResponse>), AppError> {
    payload.validate()?;

    let v1_request = CategoryRequest {
        name: payload.name.clone(),
        color: payload.color.clone(),
        icon: payload.icon.clone(),
        parent_id: payload.parent_id,
        category_type: payload.category_type.to_v1(),
        description: payload.description.clone(),
    };

    let repo = PostgresRepository { pool: pool.inner().clone() };
    let category = repo.create_category(&v1_request, &user.id).await?;

    Ok((Status::Created, Json(CategoryResponse::from_model(&category))))
}
