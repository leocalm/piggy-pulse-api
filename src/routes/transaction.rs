use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::database::transaction::TransactionRepository;
use crate::db::get_client;
use crate::error::app_error::AppError;
use crate::models::transaction::{TransactionRequest, TransactionResponse};
use deadpool_postgres::Pool;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{State, routes};
use uuid::Uuid;

#[rocket::post("/", data = "<payload>")]
pub async fn create_transaction(
    pool: &State<Pool>,
    _current_user: CurrentUser,
    payload: Json<TransactionRequest>,
) -> Result<(Status, Json<TransactionResponse>), AppError> {
    let client = get_client(pool).await?;
    let repo = PostgresRepository { client: &client };
    let tx = repo.create_transaction(&payload).await?;
    Ok((Status::Created, Json(TransactionResponse::from(&tx))))
}

#[rocket::get("/?<period_id>")]
pub async fn list_all_transactions(
    pool: &State<Pool>,
    _current_user: CurrentUser,
    period_id: Option<String>,
) -> Result<Json<Vec<TransactionResponse>>, AppError> {
    let client = get_client(pool).await?;
    let repo = PostgresRepository { client: &client };
    let txs = if let Some(period_id) = period_id {
        let uuid = Uuid::parse_str(&period_id)?;
        repo.get_transactions_for_period(&uuid).await?
    } else {
        repo.list_transactions().await?
    };
    Ok(Json(txs.iter().map(TransactionResponse::from).collect()))
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
    payload: Json<TransactionRequest>,
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
