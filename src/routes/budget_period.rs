use crate::auth::CurrentUser;
use crate::database::budget_period::BudgetPeriodRepository;
use crate::database::postgres_repository::PostgresRepository;
use crate::db::get_client;
use crate::error::app_error::AppError;
use crate::models::budget_period::{BudgetPeriodRequest, BudgetPeriodResponse};
use deadpool_postgres::Pool;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{routes, State};
use uuid::Uuid;

#[rocket::post("/", data = "<payload>")]
pub async fn create_budget_period(pool: &State<Pool>, _current_user: CurrentUser, payload: Json<BudgetPeriodRequest>) -> Result<(Status, String), AppError> {
    let client = get_client(pool).await?;
    let repo = PostgresRepository { client: &client };
    let budget_period_id = repo.create_budget_period(&payload).await?;
    Ok((Status::Created, budget_period_id.to_string()))
}

#[rocket::get("/")]
pub async fn list_budget_periods(pool: &State<Pool>, _current_user: CurrentUser) -> Result<Json<Vec<BudgetPeriodResponse>>, AppError> {
    let client = get_client(pool).await?;
    let repo = PostgresRepository { client: &client };
    let list = repo.list_budget_periods().await?;
    Ok(Json(list.iter().map(BudgetPeriodResponse::from).collect()))
}

#[rocket::get("/current")]
pub async fn get_current_budget_period(pool: &State<Pool>, _current_user: CurrentUser) -> Result<Json<BudgetPeriodResponse>, AppError> {
    let client = get_client(pool).await?;
    let repo = PostgresRepository { client: &client };
    let budget_period = repo.get_current_budget_period().await?;
    Ok(Json(BudgetPeriodResponse::from(&budget_period)))
}

#[rocket::put("/<id>", data = "<payload>")]
pub async fn put_budget_period(
    pool: &State<Pool>,
    _current_user: CurrentUser,
    id: &str,
    payload: Json<BudgetPeriodRequest>,
) -> Result<Json<BudgetPeriodResponse>, AppError> {
    let client = get_client(pool).await?;
    let repo = PostgresRepository { client: &client };
    let uuid = Uuid::parse_str(id)?;
    let budget_period = repo.update_budget_period(&uuid, &payload).await?;
    Ok(Json(BudgetPeriodResponse::from(&budget_period)))
}

#[rocket::delete("/<id>")]
pub async fn delete_budget_period(pool: &State<Pool>, _current_user: CurrentUser, id: &str) -> Result<Status, AppError> {
    let client = get_client(pool).await?;
    let repo = PostgresRepository { client: &client };
    let uuid = Uuid::parse_str(id)?;
    repo.delete_budget_period(&uuid).await?;
    Ok(Status::Ok)
}

pub fn routes() -> Vec<rocket::Route> {
    routes![
        create_budget_period,
        list_budget_periods,
        get_current_budget_period,
        put_budget_period,
        delete_budget_period
    ]
}
