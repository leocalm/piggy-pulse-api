use crate::database::postgres_repository::PostgresRepository;
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
use uuid::Uuid;

pub struct DashboardService<'a> {
    repository: &'a PostgresRepository,
    budget_period: &'a BudgetPeriod,
    transactions: Option<Arc<Vec<Transaction>>>,
    budget_categories: Option<Arc<Vec<BudgetCategory>>>,
    accounts: Option<Arc<Vec<Account>>>,
    all_transactions: Option<Arc<Vec<Transaction>>>,
}

impl<'a> DashboardService<'a> {
    pub fn new(repository: &'a PostgresRepository, budget_period: &'a BudgetPeriod) -> Self {
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

    async fn get_transactions(&mut self, user_id: &Uuid) -> Result<Arc<Vec<Transaction>>, AppError> {
        if self.transactions.is_none() {
            let data = self
                .repository
                .get_transactions_for_period(&self.budget_period.id, &Self::all_rows(), user_id)
                .await?;
            self.transactions = Some(Arc::new(data));
        }

        Ok(Arc::clone(self.transactions.as_ref().unwrap()))
    }

    async fn get_budget_categories(&mut self, user_id: &Uuid) -> Result<Arc<Vec<BudgetCategory>>, AppError> {
        if self.budget_categories.is_none() {
            let data = self.repository.list_budget_categories(&Self::all_rows(), user_id).await?;
            self.budget_categories = Some(Arc::new(data));
        }

        Ok(Arc::clone(self.budget_categories.as_ref().unwrap()))
    }

    async fn get_accounts(&mut self, user_id: &Uuid) -> Result<Arc<Vec<Account>>, AppError> {
        if self.accounts.is_none() {
            let data = self.repository.list_accounts(&Self::all_rows(), user_id).await?;
            self.accounts = Some(Arc::new(data));
        }

        Ok(Arc::clone(self.accounts.as_ref().unwrap()))
    }

    async fn get_all_transactions(&mut self, user_id: &Uuid) -> Result<Arc<Vec<Transaction>>, AppError> {
        if self.all_transactions.is_none() {
            let data = self.repository.list_transactions(&Self::all_rows(), user_id).await?;
            self.all_transactions = Some(Arc::new(data));
        }

        Ok(Arc::clone(self.all_transactions.as_ref().unwrap()))
    }

    pub async fn month_progress(&self) -> Result<MonthProgressResponse, AppError> {
        Ok(month_progress_for_period(self.budget_period))
    }

    pub async fn recent_transactions(&mut self, user_id: &Uuid) -> Result<Vec<TransactionResponse>, AppError> {
        Ok(recent_transactions_from(self.get_transactions(user_id).await?.as_ref()))
    }

    pub async fn budget_per_day(&mut self, user_id: &Uuid) -> Result<Vec<BudgetPerDayResponse>, AppError> {
        let transactions = self.get_transactions(user_id).await?;
        let accounts = self.get_accounts(user_id).await?;
        let all_transactions = self.get_all_transactions(user_id).await?;
        Ok(budget_per_day_from_data(
            self.budget_period,
            accounts.as_ref(),
            transactions.as_ref(),
            all_transactions.as_ref(),
        ))
    }

    pub async fn spent_per_category(&mut self, user_id: &Uuid) -> Result<Vec<SpentPerCategoryResponse>, AppError> {
        let transactions = self.get_transactions(user_id).await?;
        let budget_categories = self.get_budget_categories(user_id).await?;

        Ok(spent_per_category_from_data(budget_categories.as_ref(), transactions.as_ref()))
    }

    pub async fn monthly_burn_in(&mut self, user_id: &Uuid) -> Result<MonthlyBurnInResponse, AppError> {
        Ok(monthly_burn_in_from_data(
            self.budget_period,
            self.get_budget_categories(user_id).await?.as_ref(),
            self.get_transactions(user_id).await?.as_ref(),
        ))
    }

    async fn total_asset(&mut self, user_id: &Uuid) -> Result<i32, AppError> {
        let accounts = self.get_accounts(user_id).await?;
        let transactions = self.get_all_transactions(user_id).await?;

        Ok(total_asset_from_data(accounts.as_ref(), transactions.as_ref()))
    }

    pub async fn dashboard_response(&mut self, user_id: &Uuid) -> Result<DashboardResponse, AppError> {
        let recent_transactions = self.recent_transactions(user_id).await?;
        let month_progress = self.month_progress().await?;
        let budget_per_day = self.budget_per_day(user_id).await?;
        let spent_per_category = self.spent_per_category(user_id).await?;
        let monthly_burn_in = self.monthly_burn_in(user_id).await?;
        let total_asset = self.total_asset(user_id).await?;

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

fn recent_transactions_from(transactions: &[Transaction]) -> Vec<TransactionResponse> {
    transactions.iter().take(10).map(TransactionResponse::from).collect()
}

fn budget_per_day_from_data(
    budget_period: &BudgetPeriod,
    accounts: &[Account],
    transactions: &[Transaction],
    all_transactions: &[Transaction],
) -> Vec<BudgetPerDayResponse> {
    let mut data = Vec::new();
    for account in accounts.iter() {
        let mut current_date = budget_period.start_date;
        let mut balance = balance_on_date(Some(&budget_period.start_date), account, all_transactions);

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

            current_date = current_date.succ_opt().unwrap_or(budget_period.end_date);
        }
    }

    data
}

fn spent_per_category_from_data(budget_categories: &[BudgetCategory], transactions: &[Transaction]) -> Vec<SpentPerCategoryResponse> {
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
                percentage_spent: if budget_category.budgeted_value == 0 {
                    0
                } else {
                    amount_spent * 10000 / budget_category.budgeted_value
                },
            }
        })
        .collect::<Vec<_>>();

    data.sort_by(|a, b| b.percentage_spent.cmp(&a.percentage_spent));

    data
}

fn monthly_burn_in_from_data(budget_period: &BudgetPeriod, budget_categories: &[BudgetCategory], transactions: &[Transaction]) -> MonthlyBurnInResponse {
    MonthlyBurnInResponse {
        total_budget: budget_categories.iter().fold(0, |acc, bc| acc + bc.budgeted_value),
        spent_budget: transactions
            .iter()
            .filter(|tx| tx.category.category_type == CategoryType::Outgoing)
            .fold(0, |acc, tx| acc + tx.amount),
        current_day: Utc::now().naive_utc().date().signed_duration_since(budget_period.start_date).num_days() as i32,
        days_in_period: budget_period.end_date.signed_duration_since(budget_period.start_date).num_days() as i32,
    }
}

fn total_asset_from_data(accounts: &[Account], transactions: &[Transaction]) -> i32 {
    accounts.iter().fold(0, |acc, account| acc + balance_on_date(None, account, transactions))
}

fn month_progress_for_period(budget_period: &BudgetPeriod) -> MonthProgressResponse {
    let current_date = Utc::now().naive_utc().date();
    let period_start_date = budget_period.start_date;
    let period_end_date = budget_period.end_date;

    let days_in_period = period_end_date.signed_duration_since(period_start_date).num_days() as u32;
    let remaining_days = if current_date > period_end_date {
        0
    } else {
        period_end_date.signed_duration_since(current_date).num_days().try_into().unwrap_or(0)
    };
    let days_passed = days_in_period.saturating_sub(remaining_days);
    debug!("Days passed: {}", days_passed);
    let days_passed_ratio = days_passed as f32 / days_in_period as f32;
    debug!("days_passed_ratio: {}", days_passed_ratio);
    let days_passed_percentage = (100.0 * days_passed_ratio) as u32;

    MonthProgressResponse {
        current_date,
        days_in_period,
        remaining_days,
        days_passed_percentage,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{sample_account, sample_budget_category, sample_budget_period, sample_transaction};
    use chrono::Duration;

    #[test]
    fn test_recent_transactions_limit() {
        let transactions = (0..15).map(|_| sample_transaction()).collect::<Vec<_>>();
        let recent = recent_transactions_from(&transactions);
        assert_eq!(recent.len(), 10);
    }

    #[test]
    fn test_spent_per_category_percentage_calculation() {
        let category = sample_budget_category();
        let transactions = vec![sample_transaction()];
        let result = spent_per_category_from_data(&[category], &transactions);
        let expected_percentage = result[0].amount_spent * 10000 / result[0].budgeted_value;
        assert_eq!(result[0].percentage_spent, expected_percentage);
    }

    #[test]
    fn test_spent_per_category_sorting_property() {
        let mut category_high = sample_budget_category();
        category_high.budgeted_value = 1;
        let mut high_tx = sample_transaction();
        high_tx.amount = 10;

        let mut category_low = sample_budget_category();
        category_low.category.name = "Low".into();
        category_low.budgeted_value = 1000;

        let result = spent_per_category_from_data(&[category_low, category_high], &[high_tx]);
        let mut sorted = result.clone();
        sorted.sort_by(|a, b| b.percentage_spent.cmp(&a.percentage_spent));
        assert_eq!(result, sorted);
    }

    #[test]
    fn test_balance_calculation_associativity() {
        let txs = vec![sample_transaction(), sample_transaction()];
        let sum1: i32 = txs.iter().map(|t| t.amount).sum();
        let sum2 = txs.iter().fold(0, |acc, tx| acc + tx.amount);
        assert_eq!(sum1, sum2);
    }

    #[test]
    fn test_month_progress_dates_consistent() {
        let budget_period = sample_budget_period();
        let response = month_progress_for_period(&budget_period);
        let remaining = response.days_in_period - response.remaining_days;
        let calculated_end = if response.current_date > budget_period.end_date {
            budget_period.end_date
        } else {
            NaiveDate::from_ymd_opt(response.current_date.year(), response.current_date.month(), response.current_date.day())
                .unwrap()
                .checked_add_signed(Duration::days(remaining as i64))
                .unwrap()
        };
        assert_eq!(calculated_end, budget_period.end_date);
    }

    #[test]
    fn test_monthly_burn_in_current_day_calculation() {
        let budget_period = sample_budget_period();
        let response = monthly_burn_in_from_data(&budget_period, &[sample_budget_category()], &[sample_transaction()]);
        let expected_current_day = Utc::now().naive_utc().date().signed_duration_since(budget_period.start_date).num_days();
        assert_eq!(response.current_day, expected_current_day as i32);
    }

    #[test]
    fn test_monthly_burn_in_days_in_period() {
        let budget_period = sample_budget_period();
        let response = monthly_burn_in_from_data(&budget_period, &[sample_budget_category()], &[sample_transaction()]);
        let expected_days_in_period = budget_period.end_date.signed_duration_since(budget_period.start_date).num_days() as i32;
        assert_eq!(response.days_in_period, expected_days_in_period);
    }

    #[test]
    fn test_total_asset_sum_property() {
        let account = sample_account();
        let mut tx = sample_transaction();
        tx.from_account = account.clone();
        tx.user_id = account.user_id;
        let accounts = vec![account];
        let transactions = vec![tx];
        let total_asset = total_asset_from_data(&accounts, &transactions);
        let sum_transactions: i32 = transactions.iter().map(|t| t.amount).sum();
        assert_eq!(total_asset, sum_transactions);
    }
}
