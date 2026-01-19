use crate::auth::CurrentUser;
use crate::database::budget_category::BudgetCategoryRepository;
use crate::database::postgres_repository::PostgresRepository;
use crate::db::get_client;
use crate::error::app_error::AppError;
use crate::models::budget_category::{BudgetCategoryRequest, BudgetCategoryResponse};
use deadpool_postgres::Pool;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{routes, State};
use uuid::Uuid;

#[rocket::post("/", data = "<payload>")]
pub async fn create_budget_category(
    pool: &State<Pool>,
    _current_user: CurrentUser,
    payload: Json<BudgetCategoryRequest>,
) -> Result<(Status, Json<BudgetCategoryResponse>), AppError> {
    let client = get_client(pool).await?;
    let repo = PostgresRepository { client: &client };
    let bc = repo.create_budget_category(&payload).await?;
    Ok((Status::Created, Json(BudgetCategoryResponse::from(&bc))))
}

#[rocket::get("/")]
pub async fn list_all_budget_categories(pool: &State<Pool>, _current_user: CurrentUser) -> Result<Json<Vec<BudgetCategoryResponse>>, AppError> {
    let client = get_client(pool).await?;
    let repo = PostgresRepository { client: &client };
    let list = repo.list_budget_categories().await?;
    Ok(Json(list.iter().map(BudgetCategoryResponse::from).collect()))
}

#[rocket::get("/<id>")]
pub async fn get_budget_category(pool: &State<Pool>, _current_user: CurrentUser, id: &str) -> Result<Json<BudgetCategoryResponse>, AppError> {
    let client = get_client(pool).await?;
    let repo = PostgresRepository { client: &client };
    let uuid = Uuid::parse_str(id)?;
    if let Some(bc) = repo.get_budget_category_by_id(&uuid).await? {
        Ok(Json(BudgetCategoryResponse::from(&bc)))
    } else {
        Err(AppError::NotFound("Budget category not found".to_string()))
    }
}

#[rocket::delete("/<id>")]
pub async fn delete_budget_category(pool: &State<Pool>, _current_user: CurrentUser, id: &str) -> Result<Status, AppError> {
    let client = get_client(pool).await?;
    let repo = PostgresRepository { client: &client };
    let uuid = Uuid::parse_str(id)?;
    repo.delete_budget_category(&uuid).await?;
    Ok(Status::Ok)
}

#[rocket::put("/<id>", data = "<payload>")]
pub async fn put_budget_category(pool: &State<Pool>, _current_user: CurrentUser, id: &str, payload: Json<i32>) -> Result<Status, AppError> {
    let client = get_client(pool).await?;
    let repo = PostgresRepository { client: &client };
    let uuid = Uuid::parse_str(id)?;
    repo.update_budget_category_value(&uuid, &payload).await?;
    Ok(Status::Ok)
}

pub fn routes() -> Vec<rocket::Route> {
    routes![
        create_budget_category,
        list_all_budget_categories,
        get_budget_category,
        delete_budget_category,
        put_budget_category
    ]
}
