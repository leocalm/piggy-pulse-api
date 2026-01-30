use crate::database::account::AccountRepository;
use crate::database::transaction::TransactionRepository;
use crate::error::app_error::AppError;
use crate::models::account::AccountResponse;
use crate::models::currency::CurrencyResponse;
use crate::models::pagination::PaginationParams;
use crate::service::service_util::balance_on_date;
use chrono::Utc;

pub struct AccountService<'a, R>
where
    R: AccountRepository + TransactionRepository,
{
    repository: &'a R,
}

impl<'a, R> AccountService<'a, R>
where
    R: AccountRepository + TransactionRepository,
{
    pub fn new(repository: &'a R) -> Self {
        AccountService { repository }
    }

    pub async fn list_accounts(&self, pagination: Option<&PaginationParams>) -> Result<Vec<AccountResponse>, AppError> {
        let (accounts, _total) = self.repository.list_accounts(pagination).await?;
        let (transactions, _tx_total) = self.repository.list_transactions(None).await?;

        Ok(accounts
            .iter()
            .map(|a| AccountResponse {
                id: a.id,
                name: a.name.clone(),
                color: a.color.clone(),
                icon: a.icon.clone(),
                account_type: a.account_type,
                currency: CurrencyResponse::from(&a.currency),
                balance: balance_on_date(Some(&Utc::now().date_naive()), a, &transactions) as i64,
                spend_limit: a.spend_limit,
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::MockRepository;

    #[tokio::test]
    async fn test_list_accounts() {
        let repo = MockRepository {};
        let service = AccountService::new(&repo);

        let result = service.list_accounts(None).await;
        assert!(result.is_ok());

        let accounts = result.unwrap();
        assert_eq!(accounts.len(), 1);
    }

    #[tokio::test]
    async fn test_list_accounts_with_pagination() {
        let repo = MockRepository {};
        let service = AccountService::new(&repo);
        let pagination = PaginationParams {
            page: Some(1),
            limit: Some(10),
        };

        let result = service.list_accounts(Some(&pagination)).await;
        assert!(result.is_ok());
    }
}
