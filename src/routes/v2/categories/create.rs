use rocket::State;
use rocket::http::Status;
use rocket::post;
use rocket::serde::json::Json;
use sqlx::PgPool;
use validator::Validate;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::categories::{CategoryBase, CategoryResponse, CategoryStatus, CategoryType, CreateCategoryRequest};
use crate::error::app_error::AppError;
use crate::models::category::{CategoryRequest, CategoryType as V1CategoryType};

/// Map the V2 DTO CategoryType to the V1 model CategoryType.
fn to_v1_category_type(ct: CategoryType) -> V1CategoryType {
    match ct {
        CategoryType::Income => V1CategoryType::Incoming,
        CategoryType::Expense => V1CategoryType::Outgoing,
        CategoryType::Transfer => V1CategoryType::Transfer,
    }
}

/// Map the V1 model CategoryType to the V2 DTO CategoryType.
fn to_v2_category_type(ct: V1CategoryType) -> CategoryType {
    match ct {
        V1CategoryType::Incoming => CategoryType::Income,
        V1CategoryType::Outgoing => CategoryType::Expense,
        V1CategoryType::Transfer => CategoryType::Transfer,
    }
}

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
        category_type: to_v1_category_type(payload.category_type),
        description: payload.description.clone(),
    };

    let repo = PostgresRepository { pool: pool.inner().clone() };
    let category = repo.create_category(&v1_request, &user.id).await?;

    let response = CategoryResponse {
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
    };

    Ok((Status::Created, Json(response)))
}
