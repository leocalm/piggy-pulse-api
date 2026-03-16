use rocket::State;
use rocket::get;
use rocket::serde::json::Json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::accounts::{AccountDetailsResponse, AccountStatus, AccountSummaryResponse, AccountType, StabilityContext};
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

#[get("/<id>/details?<period_id>")]
pub async fn get_account_details(
    pool: &State<PgPool>,
    user: CurrentUser,
    id: &str,
    period_id: Option<String>,
) -> Result<Json<AccountDetailsResponse>, AppError> {
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid account id", e))?;
    let repo = PostgresRepository { pool: pool.inner().clone() };

    let account = repo
        .get_account_by_id(&uuid, &user.id)
        .await?
        .ok_or_else(|| AppError::NotFound("Account not found".to_string()))?;

    // Resolve period_id
    let resolved_period_id = match period_id {
        Some(ref s) => Some(Uuid::parse_str(s).map_err(|e| AppError::uuid("Invalid period id", e))?),
        None => repo.get_current_period_id(&user.id).await?,
    };

    // Build base summary
    let base = AccountSummaryResponse {
        id: account.id,
        name: account.name.clone(),
        account_type: convert_account_type(account.account_type),
        color: account.color.clone(),
        status: if account.is_archived {
            AccountStatus::Inactive
        } else {
            AccountStatus::Active
        },
        current_balance: account.balance,
        net_change_this_period: 0,
        next_transfer: None,
        balance_after_next_transfer: None,
        number_of_transactions: 0,
    };

    // If we have a period, try to get detail metrics
    let (inflow, outflow) = if let Some(pid) = &resolved_period_id {
        match repo.get_account_detail(&uuid, pid, &user.id).await {
            Ok(detail) => (detail.inflows, detail.outflows),
            Err(_) => (0, 0),
        }
    } else {
        (0, 0)
    };

    Ok(Json(AccountDetailsResponse {
        base,
        inflow,
        outflow,
        stability_context: StabilityContext {
            periods_on_target: 0,
            average_closing_balance: 0,
            highest_closing_balance: 0,
            lowest_closing_balance: 0,
            largest_single_outflow: None,
        },
        categories_breakdown: vec![],
        transactions_breakdown: vec![],
    }))
}
