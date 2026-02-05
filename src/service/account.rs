use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::models::account::{AccountBalancePerDay, AccountListResponse, AccountWithMetrics};
use crate::models::currency::CurrencyResponse;
use crate::models::dashboard::BudgetPerDayResponse;
use crate::models::pagination::CursorParams;
use std::collections::HashMap;
use uuid::Uuid;

pub struct AccountService<'a> {
    repository: &'a PostgresRepository,
}

impl<'a> AccountService<'a> {
    pub fn new(repository: &'a PostgresRepository) -> Self {
        AccountService { repository }
    }

    pub async fn list_accounts(&self, params: &CursorParams, budget_period_id: &Uuid, user_id: &Uuid) -> Result<Vec<AccountListResponse>, AppError> {
        let accounts = self.repository.list_accounts(params, budget_period_id, user_id).await?;
        if accounts.is_empty() {
            return Ok(Vec::new());
        }

        let account_ids: Vec<Uuid> = accounts.iter().map(|account| account.account.id).collect();
        let balance_per_day = self.repository.list_account_balance_per_day(&account_ids, budget_period_id, user_id).await?;

        Ok(account_responses(&accounts, &balance_per_day))
    }
}

fn account_responses(accounts: &[AccountWithMetrics], balance_per_day: &[AccountBalancePerDay]) -> Vec<AccountListResponse> {
    let mut per_day_by_account = balance_per_day_map(balance_per_day);

    accounts
        .iter()
        .map(|account| {
            let account_data = &account.account;
            let per_day = per_day_by_account.remove(&account_data.id).unwrap_or_default();

            AccountListResponse {
                id: account_data.id,
                name: account_data.name.clone(),
                color: account_data.color.clone(),
                icon: account_data.icon.clone(),
                account_type: account_data.account_type,
                currency: CurrencyResponse::from(&account_data.currency),
                balance: account.current_balance,
                spend_limit: account_data.spend_limit,
                balance_per_day: per_day,
                balance_change_this_period: account.balance_change_this_period,
                transaction_count: account.transaction_count,
            }
        })
        .collect()
}

fn balance_per_day_map(balance_per_day: &[AccountBalancePerDay]) -> HashMap<Uuid, Vec<BudgetPerDayResponse>> {
    let mut per_day_by_account: HashMap<Uuid, Vec<BudgetPerDayResponse>> = HashMap::new();

    for row in balance_per_day {
        per_day_by_account.entry(row.account_id).or_default().push(BudgetPerDayResponse {
            account_name: row.account_name.clone(),
            date: row.date.clone(),
            balance: row.balance,
        });
    }

    per_day_by_account
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::sample_account;

    #[tokio::test]
    async fn test_list_accounts() {
        let account = sample_account();
        let metrics = AccountWithMetrics {
            account: account.clone(),
            current_balance: 1200,
            balance_change_this_period: 200,
            transaction_count: 2,
        };
        let balances = vec![
            AccountBalancePerDay {
                account_id: account.id,
                account_name: account.name.clone(),
                date: "2026-02-01".to_string(),
                balance: 1000,
            },
            AccountBalancePerDay {
                account_id: account.id,
                account_name: account.name.clone(),
                date: "2026-02-02".to_string(),
                balance: 1200,
            },
        ];
        let responses = account_responses(&[metrics], &balances);
        assert_eq!(responses.len(), 1);
        assert_eq!(responses[0].id, account.id);
        assert_eq!(responses[0].balance, 1200);
        assert_eq!(responses[0].balance_change_this_period, 200);
        assert_eq!(responses[0].transaction_count, 2);
        assert_eq!(responses[0].balance_per_day.len(), 2);
    }

    #[tokio::test]
    async fn test_list_accounts_without_balance_per_day() {
        let account = sample_account();
        let metrics = AccountWithMetrics {
            account: account.clone(),
            current_balance: 1000,
            balance_change_this_period: 0,
            transaction_count: 0,
        };
        let responses = account_responses(&[metrics], &[]);
        assert_eq!(responses.len(), 1);
        assert_eq!(responses[0].balance_per_day.len(), 0);
    }
}
