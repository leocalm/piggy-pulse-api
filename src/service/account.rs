use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::models::account::{Account, AccountResponse};
use crate::models::currency::CurrencyResponse;
use crate::models::pagination::CursorParams;
use crate::models::transaction::Transaction;
use crate::service::service_util::balance_on_date;
use chrono::Utc;
use uuid::Uuid;

pub struct AccountService<'a> {
    repository: &'a PostgresRepository,
}

impl<'a> AccountService<'a> {
    pub fn new(repository: &'a PostgresRepository) -> Self {
        AccountService { repository }
    }

    pub async fn list_accounts(&self, params: &CursorParams, user_id: &Uuid) -> Result<Vec<AccountResponse>, AppError> {
        let accounts = self.repository.list_accounts(params, user_id).await?;
        let all_params = CursorParams {
            cursor: None,
            limit: Some(CursorParams::MAX_LIMIT),
        };
        let transactions = self.repository.list_transactions(&all_params, user_id).await?;

        Ok(account_responses(&accounts, &transactions))
    }
}

fn account_responses(accounts: &[Account], transactions: &[Transaction]) -> Vec<AccountResponse> {
    accounts
        .iter()
        .map(|a| AccountResponse {
            id: a.id,
            name: a.name.clone(),
            color: a.color.clone(),
            icon: a.icon.clone(),
            account_type: a.account_type,
            currency: CurrencyResponse::from(&a.currency),
            balance: balance_on_date(Some(&Utc::now().date_naive()), a, transactions) as i64,
            spend_limit: a.spend_limit,
        })
        .collect()
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
