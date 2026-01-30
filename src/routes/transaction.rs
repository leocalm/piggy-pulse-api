use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::database::transaction::TransactionRepository;
use crate::db::get_client;
use crate::error::app_error::AppError;
use crate::error::json::JsonBody;
use crate::models::pagination::{PaginatedResponse, PaginationParams};
use crate::models::transaction::{TransactionRequest, TransactionResponse};
use deadpool_postgres::Pool;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{State, routes};
use uuid::Uuid;
use validator::Validate;

#[rocket::post("/", data = "<payload>")]
pub async fn create_transaction(
    pool: &State<Pool>,
    _current_user: CurrentUser,
    payload: JsonBody<TransactionRequest>,
) -> Result<(Status, Json<TransactionResponse>), AppError> {
    payload.validate()?;

    let start = std::time::Instant::now();

    let client_start = std::time::Instant::now();
    let client = get_client(pool).await?;
    tracing::trace!("Got DB client in {:?}", client_start.elapsed());

    let repo = PostgresRepository { client: &client };

    let query_start = std::time::Instant::now();
    let tx = repo.create_transaction(&payload).await?;
    tracing::trace!("Created transaction in {:?}", query_start.elapsed());

    tracing::trace!("Total create_transaction time: {:?}", start.elapsed());
    Ok((Status::Created, Json(TransactionResponse::from(&tx))))
}

#[rocket::get("/?<period_id>&<page>&<limit>")]
pub async fn list_all_transactions(
    pool: &State<Pool>,
    _current_user: CurrentUser,
    period_id: Option<String>,
    page: Option<i64>,
    limit: Option<i64>,
) -> Result<Json<PaginatedResponse<TransactionResponse>>, AppError> {
    let client = get_client(pool).await?;
    let repo = PostgresRepository { client: &client };

    let pagination = if page.is_some() || limit.is_some() {
        Some(PaginationParams { page, limit })
    } else {
        None
    };

    let (txs, total) = if let Some(period_id) = period_id {
        let uuid = Uuid::parse_str(&period_id)?;
        repo.get_transactions_for_period(&uuid, pagination.as_ref()).await?
    } else {
        repo.list_transactions(pagination.as_ref()).await?
    };

    let responses: Vec<TransactionResponse> = txs.iter().map(TransactionResponse::from).collect();

    let paginated = if let Some(params) = pagination {
        let effective_page = params.page.unwrap_or(1);
        let effective_limit = params.effective_limit().unwrap_or(PaginationParams::DEFAULT_LIMIT);
        PaginatedResponse::new(responses, effective_page, effective_limit, total)
    } else {
        // No pagination requested - return all with metadata showing all on page 1
        PaginatedResponse::new(responses, 1, total, total)
    };

    Ok(Json(paginated))
}

#[rocket::get("/<id>")]
pub async fn get_transaction(pool: &State<Pool>, _current_user: CurrentUser, id: &str) -> Result<Json<TransactionResponse>, AppError> {
    let client = get_client(pool).await?;
    let repo = PostgresRepository { client: &client };
    let uuid = Uuid::parse_str(id)?;
    if let Some(tx) = repo.get_transaction_by_id(&uuid).await? {
        Ok(Json(TransactionResponse::from(&tx)))
    } else {
        Err(AppError::NotFound("Transaction not found".to_string()))
    }
}

#[rocket::delete("/<id>")]
pub async fn delete_transaction(pool: &State<Pool>, _current_user: CurrentUser, id: &str) -> Result<Status, AppError> {
    let client = get_client(pool).await?;
    let repo = PostgresRepository { client: &client };
    let uuid = Uuid::parse_str(id)?;
    repo.delete_transaction(&uuid).await?;
    Ok(Status::Ok)
}

#[rocket::put("/<id>", data = "<payload>")]
pub async fn put_transaction(
    pool: &State<Pool>,
    _current_user: CurrentUser,
    id: &str,
    payload: JsonBody<TransactionRequest>,
) -> Result<Json<TransactionResponse>, AppError> {
    let client = get_client(pool).await?;
    let repo = PostgresRepository { client: &client };
    let uuid = Uuid::parse_str(id)?;
    let tx = repo.update_transaction(&uuid, &payload).await?;
    Ok(Json(TransactionResponse::from(&tx)))
}

pub fn routes() -> Vec<rocket::Route> {
    routes![create_transaction, list_all_transactions, get_transaction, delete_transaction, put_transaction]
}

#[cfg(test)]
mod tests {
    use crate::{Config, build_rocket};
    use rocket::http::Status;
    use rocket::local::asynchronous::Client;

    #[rocket::async_test]
    async fn test_get_transaction_invalid_uuid() {
        let mut config = Config::default();
        config.database.url = "postgresql://test:test@localhost/test".to_string();

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");

        let response = client.get("/api/transactions/invalid-uuid").dispatch().await;

        assert_eq!(response.status(), Status::BadRequest);
    }

    #[rocket::async_test]
    async fn test_delete_transaction_invalid_uuid() {
        let mut config = Config::default();
        config.database.url = "postgresql://test:test@localhost/test".to_string();

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");

        let response = client.delete("/api/transactions/bad-id").dispatch().await;

        assert_eq!(response.status(), Status::BadRequest);
    }

    #[rocket::async_test]
    async fn test_list_transactions_invalid_period_id() {
        let mut config = Config::default();
        config.database.url = "postgresql://test:test@localhost/test".to_string();

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");

        let response = client.get("/api/transactions/?period_id=invalid-uuid").dispatch().await;

        assert_eq!(response.status(), Status::BadRequest);
    }
}
