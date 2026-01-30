use crate::auth::CurrentUser;
use crate::database::category::CategoryRepository;
use crate::database::postgres_repository::PostgresRepository;
use crate::db::get_client;
use crate::error::app_error::AppError;
use crate::models::category::{CategoryRequest, CategoryResponse};
use crate::models::pagination::{PaginatedResponse, PaginationParams};
use deadpool_postgres::Pool;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{routes, State};
use uuid::Uuid;
use validator::Validate;

#[rocket::post("/", data = "<payload>")]
pub async fn create_category(
    pool: &State<Pool>,
    _current_user: CurrentUser,
    payload: Json<CategoryRequest>,
) -> Result<(Status, Json<CategoryResponse>), AppError> {
    payload.validate()?;

    let client = get_client(pool).await?;
    let repo = PostgresRepository { client: &client };
    let category = repo.create_category(&payload).await?;
    Ok((Status::Created, Json(CategoryResponse::from(&category))))
}

#[rocket::get("/?<page>&<limit>")]
pub async fn list_all_categories(pool: &State<Pool>, _current_user: CurrentUser, page: Option<i64>, limit: Option<i64>) -> Result<Json<PaginatedResponse<CategoryResponse>>, AppError> {
    let client = get_client(pool).await?;
    let repo = PostgresRepository { client: &client };

    let pagination = if page.is_some() || limit.is_some() {
        Some(PaginationParams { page, limit })
    } else {
        None
    };

    let (categories, total) = repo.list_categories(pagination.as_ref()).await?;
    let responses: Vec<CategoryResponse> = categories.iter().map(CategoryResponse::from).collect();

    let paginated = if let Some(params) = pagination {
        let effective_page = params.page.unwrap_or(1);
        let effective_limit = params.effective_limit().unwrap_or(PaginationParams::DEFAULT_LIMIT);
        PaginatedResponse::new(responses, effective_page, effective_limit, total)
    } else {
        PaginatedResponse::new(responses, 1, total, total)
    };

    Ok(Json(paginated))
}

#[rocket::get("/<id>")]
pub async fn get_category(pool: &State<Pool>, _current_user: CurrentUser, id: &str) -> Result<Json<CategoryResponse>, AppError> {
    let client = get_client(pool).await?;
    let repo = PostgresRepository { client: &client };
    let uuid = Uuid::parse_str(id)?;
    if let Some(category) = repo.get_category_by_id(&uuid).await? {
        Ok(Json(CategoryResponse::from(&category)))
    } else {
        Err(AppError::NotFound("Category not found".to_string()))
    }
}

#[rocket::delete("/<id>")]
pub async fn delete_category(pool: &State<Pool>, _current_user: CurrentUser, id: &str) -> Result<Status, AppError> {
    let client = get_client(pool).await?;
    let repo = PostgresRepository { client: &client };
    let uuid = Uuid::parse_str(id)?;
    repo.delete_category(&uuid).await?;
    Ok(Status::Ok)
}

#[rocket::put("/<id>", data = "<payload>")]
pub async fn put_category(
    pool: &State<Pool>,
    _current_user: CurrentUser,
    id: &str,
    payload: Json<CategoryRequest>,
) -> Result<Json<CategoryResponse>, AppError> {
    let client = get_client(pool).await?;
    let repo = PostgresRepository { client: &client };
    let uuid = Uuid::parse_str(id)?;
    let category = repo.update_category(&uuid, &payload).await?;
    Ok(Json(CategoryResponse::from(&category)))
}

#[rocket::get("/not-in-budget?<page>&<limit>")]
pub async fn list_categories_not_in_budget(pool: &State<Pool>, _current_user: CurrentUser, page: Option<i64>, limit: Option<i64>) -> Result<Json<PaginatedResponse<CategoryResponse>>, AppError> {
    let client = get_client(pool).await?;
    let repo = PostgresRepository { client: &client };

    let pagination = if page.is_some() || limit.is_some() {
        Some(PaginationParams { page, limit })
    } else {
        None
    };

    let (categories, total) = repo.list_categories_not_in_budget(pagination.as_ref()).await?;
    let responses: Vec<CategoryResponse> = categories.iter().map(CategoryResponse::from).collect();

    let paginated = if let Some(params) = pagination {
        let effective_page = params.page.unwrap_or(1);
        let effective_limit = params.effective_limit().unwrap_or(PaginationParams::DEFAULT_LIMIT);
        PaginatedResponse::new(responses, effective_page, effective_limit, total)
    } else {
        PaginatedResponse::new(responses, 1, total, total)
    };

    Ok(Json(paginated))
}

pub fn routes() -> Vec<rocket::Route> {
    routes![
        create_category,
        list_all_categories,
        get_category,
        delete_category,
        put_category,
        list_categories_not_in_budget
    ]
}
