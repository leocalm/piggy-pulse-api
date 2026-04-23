use chrono::NaiveDate;
use rocket::State;
use rocket::get;
use rocket::serde::json::Json;
use sqlx::PgPool;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::transactions::EncryptedTransactionResponse;
use crate::error::app_error::AppError;
use crate::service::transaction::TransactionService;

#[get("/range?<from>&<to>")]
pub async fn list_transactions_range(
    pool: &State<PgPool>,
    user: CurrentUser,
    from: Option<String>,
    to: Option<String>,
) -> Result<Json<Vec<EncryptedTransactionResponse>>, AppError> {
    let from_str = from.ok_or_else(|| AppError::BadRequest("from is required (YYYY-MM-DD)".to_string()))?;
    let to_str = to.ok_or_else(|| AppError::BadRequest("to is required (YYYY-MM-DD)".to_string()))?;
    let from_date = NaiveDate::parse_from_str(&from_str, "%Y-%m-%d").map_err(|_| AppError::BadRequest(format!("Invalid 'from' date: {}", from_str)))?;
    let to_date = NaiveDate::parse_from_str(&to_str, "%Y-%m-%d").map_err(|_| AppError::BadRequest(format!("Invalid 'to' date: {}", to_str)))?;
    if from_date > to_date {
        return Err(AppError::BadRequest("'from' must be <= 'to'".to_string()));
    }

    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = TransactionService::new(&repo);
    Ok(Json(service.list_by_range(&user.id, from_date, to_date).await?))
}
