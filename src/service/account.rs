use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::models::account::{Account, AccountListResponse};
use crate::models::currency::CurrencyResponse;
use crate::models::pagination::CursorParams;
use crate::models::transaction::Transaction;
use crate::service::service_util::{account_involved, balance_on_date};
use chrono::{Datelike, NaiveDate, Utc};
use uuid::Uuid;

pub struct AccountService<'a> {
    repository: &'a PostgresRepository,
}

impl<'a> AccountService<'a> {
    pub fn new(repository: &'a PostgresRepository) -> Self {
        AccountService { repository }
    }

    pub async fn list_accounts(&self, params: &CursorParams, user_id: &Uuid) -> Result<Vec<AccountListResponse>, AppError> {
        let accounts = self.repository.list_accounts(params, user_id).await?;
        let all_params = CursorParams {
            cursor: None,
            limit: Some(CursorParams::MAX_LIMIT),
        };
        let transactions = self.repository.list_transactions(&all_params, user_id).await?;

        Ok(account_responses(&accounts, &transactions))
    }
}

fn account_responses(accounts: &[Account], transactions: &[Transaction]) -> Vec<AccountListResponse> {
    let now = Utc::now().date_naive();
    let month_start = month_start_date(now);
    let days_in_month = days_in_month_so_far(now, month_start);

    accounts
        .iter()
        .map(|a| {
            let current_balance = balance_on_date(None, a, transactions) as i64;
            let month_start_balance = balance_on_date(Some(&month_start), a, transactions) as i64;
            let balance_change_this_month = current_balance - month_start_balance;
            let balance_per_day = if days_in_month > 0 { balance_change_this_month / days_in_month } else { 0 };
            let transactions_count = transactions.iter().filter(|tx| account_involved(a, tx)).count() as i64;

            AccountListResponse {
                id: a.id,
                name: a.name.clone(),
                color: a.color.clone(),
                icon: a.icon.clone(),
                account_type: a.account_type,
                currency: CurrencyResponse::from(&a.currency),
                balance: balance_on_date(Some(&now), a, transactions) as i64,
                current_balance,
                balance_per_day,
                balance_change_this_month,
                transactions_count,
                spend_limit: a.spend_limit,
            }
        })
        .collect()
}

fn month_start_date(date: NaiveDate) -> NaiveDate {
    NaiveDate::from_ymd_opt(date.year(), date.month(), 1).unwrap_or(date)
}

fn days_in_month_so_far(current_date: NaiveDate, month_start: NaiveDate) -> i64 {
    (current_date.signed_duration_since(month_start).num_days() + 1).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{sample_account, sample_transaction};

    #[tokio::test]
    async fn test_list_accounts() {
        let accounts = vec![sample_account()];
        let transactions = vec![sample_transaction()];
        let responses = account_responses(&accounts, &transactions);
        assert_eq!(responses.len(), 1);
        assert_eq!(responses[0].id, accounts[0].id);
    }

    #[tokio::test]
    async fn test_list_accounts_with_cursor() {
        let accounts = vec![sample_account()];
        let responses = account_responses(&accounts, &[]);
        assert_eq!(responses.len(), 1);
    }
}
