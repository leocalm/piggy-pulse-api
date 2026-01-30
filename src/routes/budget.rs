use crate::auth::CurrentUser;
use crate::database::budget::BudgetRepository;
use crate::database::postgres_repository::PostgresRepository;
use crate::db::get_client;
use crate::error::app_error::AppError;
use crate::models::budget::{BudgetRequest, BudgetResponse};
use deadpool_postgres::Pool;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{routes, State};
use uuid::Uuid;
use validator::Validate;

#[rocket::post("/", data = "<payload>")]
pub async fn create_budget(pool: &State<Pool>, _current_user: CurrentUser, payload: Json<BudgetRequest>) -> Result<(Status, Json<BudgetResponse>), AppError> {
    payload.validate()?;

    let client = get_client(pool).await?;
    let repo = PostgresRepository { client: &client };
    let budget = repo.create_budget(&payload).await?;
    Ok((Status::Created, Json(BudgetResponse::from(&budget))))
}

#[rocket::get("/")]
pub async fn list_all_budgets(pool: &State<Pool>, _current_user: CurrentUser) -> Result<Json<Vec<BudgetResponse>>, AppError> {
    let client = get_client(pool).await?;
    let repo = PostgresRepository { client: &client };
    let budgets = repo.list_budgets().await?;
    Ok(Json(budgets.iter().map(BudgetResponse::from).collect()))
}

#[rocket::get("/<id>")]
pub async fn get_budget(pool: &State<Pool>, _current_user: CurrentUser, id: &str) -> Result<Json<BudgetResponse>, AppError> {
    let client = get_client(pool).await?;
    let repo = PostgresRepository { client: &client };
    let uuid = Uuid::parse_str(id)?;
    if let Some(budget) = repo.get_budget_by_id(&uuid).await? {
        Ok(Json(BudgetResponse::from(&budget)))
    } else {
        Err(AppError::NotFound("Budget not found".to_string()))
    }
}

#[rocket::put("/<id>", data = "<payload>")]
pub async fn put_budget(
    pool: &State<Pool>,
    _current_user: CurrentUser,
    id: &str,
    payload: Json<BudgetRequest>,
) -> Result<(Status, Json<BudgetResponse>), AppError> {
    let client = get_client(pool).await?;
    let repo = PostgresRepository { client: &client };
    let budget = repo.update_budget(&Uuid::parse_str(id)?, &payload).await?;
    Ok((Status::Ok, Json(BudgetResponse::from(&budget))))
}

#[rocket::delete("/<id>")]
pub async fn delete_budget(pool: &State<Pool>, _current_user: CurrentUser, id: &str) -> Result<Status, AppError> {
    let client = get_client(pool).await?;
    let repo = PostgresRepository { client: &client };
    let uuid = Uuid::parse_str(id)?;
    repo.delete_budget(&uuid).await?;
    Ok(Status::Ok)
}

pub fn routes() -> Vec<rocket::Route> {
    routes![create_budget, list_all_budgets, get_budget, put_budget, delete_budget]
}
