use crate::auth::CurrentUser;
use crate::database::budget_period::BudgetPeriodRepository;
use crate::database::postgres_repository::PostgresRepository;
use crate::db::get_client;
use crate::error::app_error::AppError;
use crate::models::budget_period::{BudgetPeriodRequest, BudgetPeriodResponse};
use crate::models::pagination::{PaginatedResponse, PaginationParams};
use deadpool_postgres::Pool;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{State, routes};
use uuid::Uuid;
use validator::Validate;

#[rocket::post("/", data = "<payload>")]
pub async fn create_budget_period(pool: &State<Pool>, _current_user: CurrentUser, payload: Json<BudgetPeriodRequest>) -> Result<(Status, String), AppError> {
    payload.validate()?;

    let client = get_client(pool).await?;
    let repo = PostgresRepository { client: &client };
    let budget_period_id = repo.create_budget_period(&payload).await?;
    Ok((Status::Created, budget_period_id.to_string()))
}

#[rocket::get("/?<page>&<limit>")]
pub async fn list_budget_periods(
    pool: &State<Pool>,
    _current_user: CurrentUser,
    page: Option<i64>,
    limit: Option<i64>,
) -> Result<Json<PaginatedResponse<BudgetPeriodResponse>>, AppError> {
    let client = get_client(pool).await?;
    let repo = PostgresRepository { client: &client };

    let pagination = if page.is_some() || limit.is_some() {
        Some(PaginationParams { page, limit })
    } else {
        None
    };

    let (list, total) = repo.list_budget_periods(pagination.as_ref()).await?;
    let responses: Vec<BudgetPeriodResponse> = list.iter().map(BudgetPeriodResponse::from).collect();

    let paginated = if let Some(params) = pagination {
        let effective_page = params.page.unwrap_or(1);
        let effective_limit = params.effective_limit().unwrap_or(PaginationParams::DEFAULT_LIMIT);
        PaginatedResponse::new(responses, effective_page, effective_limit, total)
    } else {
        PaginatedResponse::new(responses, 1, total, total)
    };

    Ok(Json(paginated))
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
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid budget period id", e))?;
    let budget_period = repo.update_budget_period(&uuid, &payload).await?;
    Ok(Json(BudgetPeriodResponse::from(&budget_period)))
}

#[rocket::delete("/<id>")]
pub async fn delete_budget_period(pool: &State<Pool>, _current_user: CurrentUser, id: &str) -> Result<Status, AppError> {
    let client = get_client(pool).await?;
    let repo = PostgresRepository { client: &client };
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid budget period id", e))?;
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

#[cfg(test)]
mod tests {
    use crate::{Config, build_rocket};
    use rocket::http::Status;
    use rocket::local::asynchronous::Client;

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn test_put_budget_period_invalid_uuid() {
        let mut config = Config::default();
        config.database.url = "postgresql://test:test@localhost/test".to_string();

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");

        let response = client.put("/api/budget_period/invalid").dispatch().await;

        assert_eq!(response.status(), Status::BadRequest);
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn test_delete_budget_period_invalid_uuid() {
        let mut config = Config::default();
        config.database.url = "postgresql://test:test@localhost/test".to_string();

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");

        let response = client.delete("/api/budget_period/not-valid").dispatch().await;

        assert_eq!(response.status(), Status::BadRequest);
    }
}
