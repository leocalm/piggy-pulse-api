use rocket::State;
use rocket::get;
use rocket::serde::json::Json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::accounts::BatchBalanceHistoryResponse;
use crate::error::app_error::AppError;
use crate::service::account::AccountService;

#[get("/balance-history?<periodId>")]
pub async fn get_batch_balance_history(
    pool: &State<PgPool>,
    user: CurrentUser,
    #[allow(non_snake_case)] periodId: Option<String>,
) -> Result<Json<BatchBalanceHistoryResponse>, AppError> {
    let pid = match periodId {
        Some(ref s) => Uuid::parse_str(s).map_err(|e| AppError::uuid("Invalid period id", e))?,
        None => return Err(AppError::BadRequest("periodId is required".to_string())),
    };

    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = AccountService::new(&repo);
    let response = service.get_batch_balance_history(&pid, &user.id).await?;
    Ok(Json(response))
}
