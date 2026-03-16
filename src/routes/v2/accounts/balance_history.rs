use rocket::State;
use rocket::get;
use rocket::serde::json::Json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::accounts::AccountBalanceHistoryResponse;
use crate::error::app_error::AppError;
use crate::service::account::AccountService;

#[get("/<id>/balance-history?<period_id>")]
pub async fn get_balance_history(
    pool: &State<PgPool>,
    user: CurrentUser,
    id: &str,
    period_id: Option<String>,
) -> Result<Json<AccountBalanceHistoryResponse>, AppError> {
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid account id", e))?;
    let period_uuid = match period_id {
        Some(ref s) => Some(Uuid::parse_str(s).map_err(|e| AppError::uuid("Invalid period id", e))?),
        None => None,
    };

    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = AccountService::new(&repo);

    let response = service.get_balance_history(&uuid, period_uuid, &user.id).await?;
    Ok(Json(response))
}
