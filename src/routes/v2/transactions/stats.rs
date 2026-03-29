use rocket::State;
use rocket::get;
use rocket::serde::json::Json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::transactions::TransactionStatsResponse;
use crate::error::app_error::AppError;

#[get("/stats?<periodId>")]
pub async fn get_transaction_stats(
    pool: &State<PgPool>,
    user: CurrentUser,
    #[allow(non_snake_case)] periodId: Option<String>,
) -> Result<Json<TransactionStatsResponse>, AppError> {
    let pid = match periodId {
        Some(ref s) => Uuid::parse_str(s).map_err(|e| AppError::uuid("Invalid period id", e))?,
        None => return Err(AppError::BadRequest("periodId is required".to_string())),
    };

    let repo = PostgresRepository { pool: pool.inner().clone() };
    let response = repo.get_transaction_stats(&pid, &user.id).await?;
    Ok(Json(response))
}
