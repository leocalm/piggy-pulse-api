use rocket::State;
use rocket::get;
use rocket::serde::json::Json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::accounts::{AccountBalanceHistoryPoint, AccountBalanceHistoryResponse};
use crate::dto::common::Date;
use crate::error::app_error::AppError;

#[get("/<id>/balance-history?<period_id>")]
pub async fn get_balance_history(
    pool: &State<PgPool>,
    user: CurrentUser,
    id: &str,
    period_id: Option<String>,
) -> Result<Json<AccountBalanceHistoryResponse>, AppError> {
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid account id", e))?;
    let repo = PostgresRepository { pool: pool.inner().clone() };

    // Verify account exists and belongs to user
    repo.get_account_by_id(&uuid, &user.id)
        .await?
        .ok_or_else(|| AppError::NotFound("Account not found".to_string()))?;

    // Resolve period
    let resolved_period_id = match period_id {
        Some(ref s) => Some(Uuid::parse_str(s).map_err(|e| AppError::uuid("Invalid period id", e))?),
        None => repo.get_current_period_id(&user.id).await?,
    };

    // If we have a period, get the balance history using period dates
    if let Some(pid) = resolved_period_id {
        let period = repo.get_budget_period(&pid, &user.id).await?;
        let points = repo.get_account_balance_history(&uuid, period.start_date, period.end_date, &user.id).await?;

        let response: Vec<AccountBalanceHistoryPoint> = points
            .into_iter()
            .map(|p| {
                let date = chrono::NaiveDate::parse_from_str(&p.date, "%Y-%m-%d").unwrap_or_default();
                AccountBalanceHistoryPoint {
                    date: Date(date),
                    balance: p.balance,
                    transaction_count: 0,
                }
            })
            .collect();

        return Ok(Json(response));
    }

    // No period available, return empty history
    Ok(Json(vec![]))
}
