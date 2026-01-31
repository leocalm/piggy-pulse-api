use crate::auth::CurrentUser;
use crate::database::budget::BudgetRepository;
use crate::database::postgres_repository::PostgresRepository;
use crate::db::get_client;
use crate::error::app_error::AppError;
use crate::models::budget::{BudgetRequest, BudgetResponse};
use crate::models::pagination::{PaginatedResponse, PaginationParams};
use deadpool_postgres::Pool;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{State, routes};
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

#[rocket::get("/?<page>&<limit>")]
pub async fn list_all_budgets(
    pool: &State<Pool>,
    _current_user: CurrentUser,
    page: Option<i64>,
    limit: Option<i64>,
) -> Result<Json<PaginatedResponse<BudgetResponse>>, AppError> {
    let client = get_client(pool).await?;
    let repo = PostgresRepository { client: &client };

    let pagination = if page.is_some() || limit.is_some() {
        Some(PaginationParams { page, limit })
    } else {
        None
    };

    let (budgets, total) = repo.list_budgets(pagination.as_ref()).await?;
    let responses: Vec<BudgetResponse> = budgets.iter().map(BudgetResponse::from).collect();

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
pub async fn get_budget(pool: &State<Pool>, _current_user: CurrentUser, id: &str) -> Result<Json<BudgetResponse>, AppError> {
    let client = get_client(pool).await?;
    let repo = PostgresRepository { client: &client };
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid budget id", e))?;
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
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid budget id", e))?;
    let budget = repo.update_budget(&uuid, &payload).await?;
    Ok((Status::Ok, Json(BudgetResponse::from(&budget))))
}

#[rocket::delete("/<id>")]
pub async fn delete_budget(pool: &State<Pool>, _current_user: CurrentUser, id: &str) -> Result<Status, AppError> {
    let client = get_client(pool).await?;
    let repo = PostgresRepository { client: &client };
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid budget id", e))?;
    repo.delete_budget(&uuid).await?;
    Ok(Status::Ok)
}

pub fn routes() -> Vec<rocket::Route> {
    routes![create_budget, list_all_budgets, get_budget, put_budget, delete_budget]
}

#[cfg(test)]
mod tests {
    use crate::{Config, build_rocket};
    use rocket::http::{ContentType, Status};
    use rocket::local::asynchronous::Client;

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn test_create_budget_validation_short_name() {
        let mut config = Config::default();
        config.database.url = "postgresql://test:test@localhost/test".to_string();

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");

        let invalid_payload = serde_json::json!({
            "name": "AB",  // Too short (min 3)
            "start_day": 1
        });

        let response = client
            .post("/api/budgets/")
            .header(ContentType::JSON)
            .body(invalid_payload.to_string())
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::BadRequest);
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn test_create_budget_validation_invalid_start_day() {
        let mut config = Config::default();
        config.database.url = "postgresql://test:test@localhost/test".to_string();

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");

        let invalid_payload = serde_json::json!({
            "name": "Test Budget",
            "start_day": 32  // Invalid (max 31)
        });

        let response = client
            .post("/api/budgets/")
            .header(ContentType::JSON)
            .body(invalid_payload.to_string())
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::BadRequest);
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn test_get_budget_invalid_uuid() {
        let mut config = Config::default();
        config.database.url = "postgresql://test:test@localhost/test".to_string();

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");

        let response = client.get("/api/budgets/not-a-uuid").dispatch().await;

        assert_eq!(response.status(), Status::BadRequest);
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn test_delete_budget_invalid_uuid() {
        let mut config = Config::default();
        config.database.url = "postgresql://test:test@localhost/test".to_string();

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");

        let response = client.delete("/api/budgets/invalid").dispatch().await;

        assert_eq!(response.status(), Status::BadRequest);
    }
}
