use rocket::State;
use rocket::get;
use rocket::serde::json::Json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::accounts::{AccountStatus, AccountSummaryListResponse, AccountSummaryResponse, AccountType};
use crate::dto::common::PaginatedResponse;
use crate::error::app_error::AppError;
use crate::models::account::AccountType as ModelAccountType;

fn convert_account_type(t: ModelAccountType) -> AccountType {
    match t {
        ModelAccountType::Checking => AccountType::Checking,
        ModelAccountType::Savings => AccountType::Savings,
        ModelAccountType::CreditCard => AccountType::CreditCard,
        ModelAccountType::Wallet => AccountType::Wallet,
        ModelAccountType::Allowance => AccountType::Allowance,
    }
}

#[get("/summary?<period_id>&<cursor>&<limit>")]
pub async fn list_account_summaries(
    pool: &State<PgPool>,
    user: CurrentUser,
    period_id: Option<String>,
    cursor: Option<String>,
    limit: Option<u32>,
) -> Result<Json<AccountSummaryListResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };

    let cursor_uuid = match cursor {
        Some(ref s) if !s.is_empty() => Some(Uuid::parse_str(s).map_err(|e| AppError::uuid("Invalid cursor", e))?),
        _ => None,
    };
    let effective_limit = limit.unwrap_or(50).min(200) as i64;

    // Resolve period_id: use provided, else fall back to current period
    let resolved_period_id = match period_id {
        Some(ref s) => Some(Uuid::parse_str(s).map_err(|e| AppError::uuid("Invalid period id", e))?),
        None => repo.get_current_period_id(&user.id).await?,
    };

    let (mut accounts, total_count) = repo
        .list_accounts_summary_v2(cursor_uuid, effective_limit, resolved_period_id.as_ref(), &user.id)
        .await?;

    let has_more = accounts.len() as i64 > effective_limit;
    if has_more {
        accounts.truncate(effective_limit as usize);
    }
    let next_cursor = if has_more { accounts.last().map(|a| a.account.id.to_string()) } else { None };

    let data: Vec<AccountSummaryResponse> = accounts
        .iter()
        .map(|m| AccountSummaryResponse {
            id: m.account.id,
            name: m.account.name.clone(),
            account_type: convert_account_type(m.account.account_type),
            color: m.account.color.clone(),
            status: if m.account.is_archived {
                AccountStatus::Inactive
            } else {
                AccountStatus::Active
            },
            current_balance: m.current_balance,
            net_change_this_period: m.balance_change_this_period,
            next_transfer: None,
            balance_after_next_transfer: None,
            number_of_transactions: m.transaction_count,
        })
        .collect();

    Ok(Json(PaginatedResponse {
        data,
        total_count,
        has_more,
        next_cursor,
    }))
}
