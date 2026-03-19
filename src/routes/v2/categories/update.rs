use rocket::State;
use rocket::put;
use rocket::serde::json::Json;
use sqlx::PgPool;
use uuid::Uuid;
use validator::Validate;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::categories::{CategoryBase, CategoryResponse, CategoryStatus, UpdateCategoryRequest};
use crate::error::app_error::AppError;
use crate::models::category::CategoryRequest;

use super::create::{to_v1_category_type, to_v2_category_type};

#[put("/<id>", data = "<payload>")]
pub async fn update_category(
    pool: &State<PgPool>,
    user: CurrentUser,
    id: &str,
    payload: Json<UpdateCategoryRequest>,
) -> Result<Json<CategoryResponse>, AppError> {
    payload.validate()?;

    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid category id", e))?;
    let repo = PostgresRepository { pool: pool.inner().clone() };

    let v1_request = CategoryRequest {
        name: payload.name.clone(),
        color: payload.color.clone(),
        icon: payload.icon.clone(),
        parent_id: payload.parent_id,
        category_type: to_v1_category_type(payload.category_type),
        description: payload.description.clone(),
    };

    let category = repo.update_category(&uuid, &v1_request, &user.id).await?;

    Ok(Json(CategoryResponse {
        base: CategoryBase {
            id: category.id,
            name: category.name,
            category_type: to_v2_category_type(category.category_type),
            icon: category.icon,
            color: category.color,
            parent_id: category.parent_id,
            status: if category.is_archived {
                CategoryStatus::Inactive
            } else {
                CategoryStatus::Active
            },
        },
        description: category.description,
    }))
}
