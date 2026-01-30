use crate::database::account::AccountRepository;
use crate::database::budget_category::BudgetCategoryRepository;
use crate::database::transaction::TransactionRepository;
use crate::error::app_error::AppError;
use crate::models::account::Account;
use crate::models::budget_category::BudgetCategory;
use crate::models::budget_period::BudgetPeriod;
use crate::models::category::CategoryType;
use crate::models::dashboard::{BudgetPerDayResponse, DashboardResponse, MonthProgressResponse, MonthlyBurnInResponse, SpentPerCategoryResponse};
use crate::models::transaction::{Transaction, TransactionResponse};
use crate::service::service_util::{account_involved, add_transaction, balance_on_date};
use chrono::prelude::*;
use std::sync::Arc;
use tracing::debug;

pub struct DashboardService<'a, R>
where
    R: TransactionRepository + BudgetCategoryRepository + AccountRepository,
{
    repository: &'a R,
    budget_period: &'a BudgetPeriod,
    transactions: Option<Arc<Vec<Transaction>>>,
    budget_categories: Option<Arc<Vec<BudgetCategory>>>,
    accounts: Option<Arc<Vec<Account>>>,
    all_transactions: Option<Arc<Vec<Transaction>>>,
}

impl<'a, R> DashboardService<'a, R>
where
    R: TransactionRepository + BudgetCategoryRepository + AccountRepository,
{
    pub fn new(repository: &'a R, budget_period: &'a BudgetPeriod) -> Self {
        Self {
            repository,
            budget_period,
            transactions: None,
            budget_categories: None,
            accounts: None,
            all_transactions: None,
        }
    }

    async fn get_transactions(&mut self) -> Result<Arc<Vec<Transaction>>, AppError> {
        if self.transactions.is_none() {
            let (data, _total) = self.repository.get_transactions_for_period(&self.budget_period.id, None).await?;
            self.transactions = Some(Arc::new(data));
        }

        Ok(Arc::clone(self.transactions.as_ref().unwrap()))
    }

    async fn get_budget_categories(&mut self) -> Result<Arc<Vec<BudgetCategory>>, AppError> {
        if self.budget_categories.is_none() {
            let (data, _total) = self.repository.list_budget_categories(None).await?;
            self.budget_categories = Some(Arc::new(data));
        }

        Ok(Arc::clone(self.budget_categories.as_ref().unwrap()))
    }

    async fn get_accounts(&mut self) -> Result<Arc<Vec<Account>>, AppError> {
        if self.accounts.is_none() {
            let (data, _total) = self.repository.list_accounts(None).await?;
            self.accounts = Some(Arc::new(data));
        }

        Ok(Arc::clone(self.accounts.as_ref().unwrap()))
    }

    async fn get_all_transactions(&mut self) -> Result<Arc<Vec<Transaction>>, AppError> {
        if self.all_transactions.is_none() {
            let (data, _total) = self.repository.list_transactions(None).await?;
            self.all_transactions = Some(Arc::new(data));
        }

        Ok(Arc::clone(self.all_transactions.as_ref().unwrap()))
    }

    pub async fn month_progress(&self) -> Result<MonthProgressResponse, AppError> {
        let current_date = Utc::now().naive_utc().date();
        let period_start_date = self.budget_period.start_date;
        let period_end_date = self.budget_period.end_date;

        let days_in_period = period_end_date.signed_duration_since(period_start_date).num_days() as u32;
        let remaining_days = period_end_date.signed_duration_since(current_date).num_days() as u32;
        let days_passed = days_in_period - remaining_days;
        debug!("Days passed: {}", days_passed);
        let days_passed_ratio = days_passed as f32 / days_in_period as f32;
        debug!("days_passed_ratio: {}", days_passed_ratio);
        let days_passed_percentage = (100.0 * days_passed_ratio) as u32;

        Ok(MonthProgressResponse {
            current_date,
            days_in_period,
            remaining_days,
            days_passed_percentage,
        })
    }

    pub async fn recent_transactions(&mut self) -> Result<Vec<TransactionResponse>, AppError> {
        Ok(self.get_transactions().await?.iter().take(10).map(TransactionResponse::from).collect())
    }

    pub async fn budget_per_day(&mut self) -> Result<Vec<BudgetPerDayResponse>, AppError> {
        let mut data = Vec::new();
        let transactions = self.get_transactions().await?;
        let accounts = self.get_accounts().await?;
        let all_transactions = self.get_all_transactions().await?;
        let start_date = self.budget_period.start_date;
        let end_date = self.budget_period.end_date;

        for account in accounts.iter() {
            let mut current_date = start_date;
            let mut balance = balance_on_date(Some(&start_date), account, &all_transactions);

            while current_date <= Utc::now().date_naive() {
                balance = transactions
                    .iter()
                    .filter(|tx| account_involved(account, tx) && tx.occurred_at == current_date)
                    .fold(balance, |acc, tx| acc + add_transaction(tx, account));

                data.push(BudgetPerDayResponse {
                    account_name: account.name.clone(),
                    date: current_date.to_string(),
                    balance,
                });

                current_date = current_date.succ_opt().unwrap_or(end_date);
            }
        }

        Ok(data)
    }

    pub async fn spent_per_category(&mut self) -> Result<Vec<SpentPerCategoryResponse>, AppError> {
        let transactions = self.get_transactions().await?;
        let budget_categories = self.get_budget_categories().await?;

        let mut data = budget_categories
            .iter()
            .map(|budget_category| {
                let amount_spent = transactions
                    .iter()
                    .filter(|tx| tx.category.id == budget_category.category.id)
                    .fold(0, |acc, tx| acc + tx.amount);
                SpentPerCategoryResponse {
                    category_name: budget_category.category.name.clone(),
                    budgeted_value: budget_category.budgeted_value,
                    amount_spent,
                    percentage_spent: amount_spent * 10000 / budget_category.budgeted_value,
                }
            })
            .collect::<Vec<_>>();

        data.sort_by(|a, b| b.percentage_spent.cmp(&a.percentage_spent));

        Ok(data)
    }

    pub async fn monthly_burn_in(&mut self) -> Result<MonthlyBurnInResponse, AppError> {
        Ok(MonthlyBurnInResponse {
            total_budget: self.get_budget_categories().await?.iter().fold(0, |acc, bc| acc + bc.budgeted_value),
            spent_budget: self
                .get_transactions()
                .await?
                .iter()
                .filter(|tx| tx.category.category_type == CategoryType::Outgoing)
                .fold(0, |acc, tx| acc + tx.amount),
            current_day: Utc::now().naive_utc().date().signed_duration_since(self.budget_period.start_date).num_days() as i32,
            days_in_period: self.budget_period.end_date.signed_duration_since(self.budget_period.start_date).num_days() as i32,
        })
    }

    async fn total_asset(&mut self) -> Result<i32, AppError> {
        let accounts = self.get_accounts().await?;
        let transactions = self.get_all_transactions().await?;

        Ok(accounts.iter().fold(0, |acc, account| acc + balance_on_date(None, account, &transactions)))
    }

    pub async fn dashboard_response(&mut self) -> Result<DashboardResponse, AppError> {
        let recent_transactions = self.recent_transactions().await?;
        let month_progress = self.month_progress().await?;
        let budget_per_day = self.budget_per_day().await?;
        let spent_per_category = self.spent_per_category().await?;
        let monthly_burn_in = self.monthly_burn_in().await?;
        let total_asset = self.total_asset().await?;

        Ok(DashboardResponse {
            budget_per_day,
            spent_per_category,
            monthly_burn_in,
            month_progress,
            recent_transactions,
            total_asset,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::category::Category;
    use crate::test_utils::MockRepository;
    use uuid::Uuid;

    #[test]
    fn test_total_asset() -> Result<(), AppError> {
        let budget_period = BudgetPeriod::default();
        let repository = MockRepository {};

        let first_account = Account {
            id: Uuid::new_v4(),
            balance: 10000,
            ..Account::default()
        };

        let mut dashboard_service = DashboardService::new(&repository, &budget_period);
        dashboard_service.accounts = Some(Arc::new(vec![
            first_account.clone(),
            Account {
                id: Uuid::new_v4(),
                balance: 20000,
                ..Account::default()
            },
        ]));
        dashboard_service.all_transactions = Some(Arc::new(vec![Transaction {
            amount: 3000,
            from_account: first_account,
            category: Category {
                category_type: CategoryType::Outgoing,
                ..Category::default()
            },
            ..Transaction::default()
        }]));

        let result = tokio::runtime::Runtime::new().unwrap().block_on(dashboard_service.total_asset())?;
        assert_eq!(27000, result);

        Ok(())
    }
}
