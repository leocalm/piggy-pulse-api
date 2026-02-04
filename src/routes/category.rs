use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::models::category::{CategoryRequest, CategoryResponse};
use crate::models::pagination::{CursorPaginatedResponse, CursorParams};
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{State, delete, get, post, put};
use rocket_okapi::openapi;
use sqlx::PgPool;
use uuid::Uuid;
use validator::Validate;

/// Create a new category
#[openapi(tag = "Categories")]
#[post("/", data = "<payload>")]
pub async fn create_category(
    pool: &State<PgPool>,
    current_user: CurrentUser,
    payload: Json<CategoryRequest>,
) -> Result<(Status, Json<CategoryResponse>), AppError> {
    payload.validate()?;

    let repo = PostgresRepository { pool: pool.inner().clone() };
    let category = repo.create_category(&payload, &current_user.id).await?;
    Ok((Status::Created, Json(CategoryResponse::from(&category))))
}

/// List all categories with cursor-based pagination
#[openapi(tag = "Categories")]
#[get("/?<cursor>&<limit>")]
pub async fn list_all_categories(
    pool: &State<PgPool>,
    current_user: CurrentUser,
    cursor: Option<String>,
    limit: Option<i64>,
) -> Result<Json<CursorPaginatedResponse<CategoryResponse>>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let params = CursorParams::from_query(cursor, limit)?;

    let categories = repo.list_categories(&params, &current_user.id).await?;
    let responses: Vec<CategoryResponse> = categories.iter().map(CategoryResponse::from).collect();
    Ok(Json(CursorPaginatedResponse::from_rows(responses, params.effective_limit(), |r| r.id)))
}

/// Get a category by ID
#[openapi(tag = "Categories")]
#[get("/<id>")]
pub async fn get_category(pool: &State<PgPool>, current_user: CurrentUser, id: &str) -> Result<Json<CategoryResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid category id", e))?;
    if let Some(category) = repo.get_category_by_id(&uuid, &current_user.id).await? {
        Ok(Json(CategoryResponse::from(&category)))
    } else {
        Err(AppError::NotFound("Category not found".to_string()))
    }
}

/// Delete a category by ID
#[openapi(tag = "Categories")]
#[delete("/<id>")]
pub async fn delete_category(pool: &State<PgPool>, current_user: CurrentUser, id: &str) -> Result<Status, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid category id", e))?;
    repo.delete_category(&uuid, &current_user.id).await?;
    Ok(Status::Ok)
}

/// Update a category by ID
#[openapi(tag = "Categories")]
#[put("/<id>", data = "<payload>")]
pub async fn put_category(
    pool: &State<PgPool>,
    current_user: CurrentUser,
    id: &str,
    payload: Json<CategoryRequest>,
) -> Result<Json<CategoryResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid category id", e))?;
    let category = repo.update_category(&uuid, &payload, &current_user.id).await?;
    Ok(Json(CategoryResponse::from(&category)))
}

/// List outgoing categories not yet associated with a budget
#[openapi(tag = "Categories")]
#[get("/not-in-budget?<cursor>&<limit>")]
pub async fn list_categories_not_in_budget(
    pool: &State<PgPool>,
    current_user: CurrentUser,
    cursor: Option<String>,
    limit: Option<i64>,
) -> Result<Json<CursorPaginatedResponse<CategoryResponse>>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let params = CursorParams::from_query(cursor, limit)?;

    let categories = repo.list_categories_not_in_budget(&params, &current_user.id).await?;
    let responses: Vec<CategoryResponse> = categories.iter().map(CategoryResponse::from).collect();
    Ok(Json(CursorPaginatedResponse::from_rows(responses, params.effective_limit(), |r| r.id)))
}

pub fn routes() -> (Vec<rocket::Route>, okapi::openapi3::OpenApi) {
    rocket_okapi::openapi_get_routes_spec![
        create_category,
        list_all_categories,
        get_category,
        delete_category,
        put_category,
        list_categories_not_in_budget
    ]
}

#[cfg(test)]
mod tests {
    use crate::{Config, build_rocket};
    use rocket::http::{ContentType, Status};
    use rocket::local::asynchronous::Client;

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn test_create_category_validation_error() {
        let mut config = Config::default();
        config.database.url = "postgresql://test:test@localhost/test".to_string();

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");

        let invalid_payload = serde_json::json!({
            "name": "AB",  // Too short
            "color": "#000",
            "icon": "icon",
            "category_type": "Outgoing"
        });

        let response = client
            .post("/api/categories/")
            .header(ContentType::JSON)
            .body(invalid_payload.to_string())
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::BadRequest);
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn test_get_category_invalid_uuid() {
        let mut config = Config::default();
        config.database.url = "postgresql://test:test@localhost/test".to_string();

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");

        let response = client.get("/api/categories/invalid-id").dispatch().await;

        assert_eq!(response.status(), Status::BadRequest);
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn test_delete_category_invalid_uuid() {
        let mut config = Config::default();
        config.database.url = "postgresql://test:test@localhost/test".to_string();

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");

        let response = client.delete("/api/categories/bad-uuid").dispatch().await;

        assert_eq!(response.status(), Status::BadRequest);
    }
}
