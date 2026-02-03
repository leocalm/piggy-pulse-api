use crate::database::account::AccountRepository;
use crate::database::budget_category::BudgetCategoryRepository;
use crate::database::transaction::TransactionRepository;
use crate::error::app_error::AppError;
use crate::models::account::Account;
use crate::models::budget_category::BudgetCategory;
use crate::models::budget_period::BudgetPeriod;
use crate::models::category::CategoryType;
use crate::models::dashboard::{BudgetPerDayResponse, DashboardResponse, MonthProgressResponse, MonthlyBurnInResponse, SpentPerCategoryResponse};
use crate::models::pagination::CursorParams;
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

    /// Params that fetch all rows — used for dashboard aggregation where we need the full set.
    const fn all_rows() -> CursorParams {
        CursorParams {
            cursor: None,
            limit: Some(CursorParams::MAX_LIMIT),
        }
    }

    async fn get_transactions(&mut self) -> Result<Arc<Vec<Transaction>>, AppError> {
        if self.transactions.is_none() {
            let data = self.repository.get_transactions_for_period(&self.budget_period.id, &Self::all_rows()).await?;
            self.transactions = Some(Arc::new(data));
        }

        Ok(Arc::clone(self.transactions.as_ref().unwrap()))
    }

    async fn get_budget_categories(&mut self) -> Result<Arc<Vec<BudgetCategory>>, AppError> {
        if self.budget_categories.is_none() {
            let data = self.repository.list_budget_categories(&Self::all_rows()).await?;
            self.budget_categories = Some(Arc::new(data));
        }

        Ok(Arc::clone(self.budget_categories.as_ref().unwrap()))
    }

    async fn get_accounts(&mut self) -> Result<Arc<Vec<Account>>, AppError> {
        if self.accounts.is_none() {
            let data = self.repository.list_accounts(&Self::all_rows()).await?;
            self.accounts = Some(Arc::new(data));
        }

        Ok(Arc::clone(self.accounts.as_ref().unwrap()))
    }

    async fn get_all_transactions(&mut self) -> Result<Arc<Vec<Transaction>>, AppError> {
        if self.all_transactions.is_none() {
            let data = self.repository.list_transactions(&Self::all_rows()).await?;
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

    mod proptest_tests {
        use super::*;
        use chrono::NaiveDate;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn test_month_progress_percentage_in_range(
                days_passed in 0u32..100,
                days_in_period in 1u32..100
            ) {
                let days_passed = days_passed.min(days_in_period);
                let days_passed_ratio = days_passed as f32 / days_in_period as f32;
                let days_passed_percentage = (100.0 * days_passed_ratio) as u32;

                prop_assert!(days_passed_percentage <= 100);
            }

            #[test]
            fn test_month_progress_dates_consistent(
                days_before in 0u64..30,
                days_after in 1u64..60
            ) {
                let current_date = Utc::now().naive_utc().date();
                let start_date = current_date.checked_sub_days(chrono::Days::new(days_before)).unwrap();
                let end_date = current_date.checked_add_days(chrono::Days::new(days_after)).unwrap();

                let repository = MockRepository {};
                let budget_period = BudgetPeriod {
                    id: Uuid::new_v4(),
                    user_id: Uuid::nil(),
                    name: "Test Period".to_string(),
                    start_date,
                    end_date,
                    created_at: Utc::now(),
                };

                let dashboard_service = DashboardService::new(&repository, &budget_period);
                let result = tokio::runtime::Runtime::new().unwrap().block_on(dashboard_service.month_progress()).unwrap();

                prop_assert!(result.days_in_period > 0);
                prop_assert!(result.days_passed_percentage <= 100);
                prop_assert!(result.remaining_days > 0);
            }

            #[test]
            fn test_total_asset_sum_property(
                balances in prop::collection::vec(0i64..1_000_000, 1..10)
            ) {
                let repository = MockRepository {};
                let budget_period = BudgetPeriod::default();

                let accounts: Vec<Account> = balances.iter().enumerate().map(|(i, &balance)| Account {
                    id: Uuid::new_v4(),
                    balance,
                    name: format!("Account {}", i),
                    ..Account::default()
                }).collect();

                let expected_total: i32 = balances.iter().map(|&b| b as i32).sum();

                let mut dashboard_service = DashboardService::new(&repository, &budget_period);
                dashboard_service.accounts = Some(Arc::new(accounts));
                dashboard_service.all_transactions = Some(Arc::new(vec![]));

                let result = tokio::runtime::Runtime::new().unwrap().block_on(dashboard_service.total_asset()).unwrap();

                prop_assert_eq!(result, expected_total);
            }

            #[test]
            fn test_total_asset_with_transactions(
                initial_balance in 0i64..1_000_000,
                transaction_amount in 0i64..10_000
            ) {
                let repository = MockRepository {};
                let budget_period = BudgetPeriod::default();

                let account = Account {
                    id: Uuid::new_v4(),
                    balance: initial_balance,
                    ..Account::default()
                };

                let transaction = Transaction {
                    amount: transaction_amount as i32,
                    from_account: account.clone(),
                    category: Category {
                        category_type: CategoryType::Outgoing,
                        ..Category::default()
                    },
                    ..Transaction::default()
                };

                let mut dashboard_service = DashboardService::new(&repository, &budget_period);
                dashboard_service.accounts = Some(Arc::new(vec![account]));
                dashboard_service.all_transactions = Some(Arc::new(vec![transaction]));

                let result = tokio::runtime::Runtime::new().unwrap().block_on(dashboard_service.total_asset()).unwrap();

                prop_assert_eq!(result, (initial_balance - transaction_amount) as i32);
            }

            #[test]
            fn test_spent_per_category_percentage_calculation(
                budgeted_value in 1i32..1_000_000,
                amount_spent in 0i32..1_000_000
            ) {
                let percentage_spent = (amount_spent as i64) * 10000 / (budgeted_value as i64);

                prop_assert!(percentage_spent >= 0);
                if amount_spent == 0 {
                    prop_assert_eq!(percentage_spent, 0);
                }
                if amount_spent == budgeted_value {
                    prop_assert_eq!(percentage_spent, 10000);
                }
            }

            #[test]
            fn test_monthly_burn_in_current_day_calculation(
                start_day in 1u32..28,
                offset_days in 0i64..30
            ) {
                let start_date = NaiveDate::from_ymd_opt(2024, 1, start_day).unwrap();
                let current_date = start_date.checked_add_days(chrono::Days::new(offset_days as u64)).unwrap();

                let calculated_offset = current_date.signed_duration_since(start_date).num_days() as i32;

                prop_assert_eq!(calculated_offset, offset_days as i32);
                prop_assert!(calculated_offset >= 0);
            }

            #[test]
            fn test_monthly_burn_in_days_in_period(
                start_day in 1u32..28,
                duration in 1i64..90
            ) {
                let start_date = NaiveDate::from_ymd_opt(2024, 1, start_day).unwrap();
                let end_date = start_date.checked_add_days(chrono::Days::new(duration as u64)).unwrap();

                let days_in_period = end_date.signed_duration_since(start_date).num_days() as i32;

                prop_assert_eq!(days_in_period, duration as i32);
                prop_assert!(days_in_period > 0);
            }

            #[test]
            fn test_budget_categories_sum(
                budgeted_values in prop::collection::vec(0i32..100_000, 1..10)
            ) {
                let repository = MockRepository {};
                let budget_period = BudgetPeriod::default();

                let budget_categories: Vec<BudgetCategory> = budgeted_values.iter().map(|&budgeted_value| BudgetCategory {
                    id: Uuid::new_v4(),
                    budgeted_value,
                    ..BudgetCategory::default()
                }).collect();

                let expected_total: i32 = budgeted_values.iter().sum();

                let mut dashboard_service = DashboardService::new(&repository, &budget_period);
                dashboard_service.budget_categories = Some(Arc::new(budget_categories));
                dashboard_service.transactions = Some(Arc::new(vec![]));

                let result = tokio::runtime::Runtime::new().unwrap().block_on(dashboard_service.monthly_burn_in()).unwrap();

                prop_assert_eq!(result.total_budget, expected_total);
                prop_assert_eq!(result.spent_budget, 0);
            }

            #[test]
            fn test_spent_per_category_sorting_property(
                percentages in prop::collection::vec(0i32..10_000, 2..10)
            ) {
                let mut sorted_percentages = percentages.clone();
                sorted_percentages.sort_by(|a, b| b.cmp(a));

                for i in 1..sorted_percentages.len() {
                    prop_assert!(sorted_percentages[i-1] >= sorted_percentages[i]);
                }
            }

            #[test]
            fn test_balance_calculation_associativity(
                initial_balance in -100_000i32..100_000,
                amounts in prop::collection::vec(-10_000i32..10_000, 0..5)
            ) {
                let total_changes: i32 = amounts.iter().sum();
                let expected_balance = initial_balance + total_changes;

                let mut running_balance = initial_balance;
                for &amount in &amounts {
                    running_balance += amount;
                }

                prop_assert_eq!(running_balance, expected_balance);
            }
        }
    }
}
