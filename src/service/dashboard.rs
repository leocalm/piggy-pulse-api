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
use crate::util::util::{account_involved, add_transaction, balance_on_date};
use chrono::prelude::*;
use deadpool_postgres::Client;
use tracing::debug;

pub struct DashboardService<'a, R>
where
    R: TransactionRepository + BudgetCategoryRepository + AccountRepository,
{
    repository: &'a R,
    budget_period: &'a BudgetPeriod,
    transactions: Option<Vec<Transaction>>,
    budget_categories: Option<Vec<BudgetCategory>>,
    accounts: Option<Vec<Account>>,
    all_transactions: Option<Vec<Transaction>>,
}

impl<'a, R> DashboardService<'a, R>
where
    R: TransactionRepository + BudgetCategoryRepository + AccountRepository,
{
    pub fn new(_client: &'a Client, repository: &'a R, budget_period: &'a BudgetPeriod) -> Self {
        Self {
            repository,
            budget_period,
            transactions: None,
            budget_categories: None,
            accounts: None,
            all_transactions: None,
        }
    }

    async fn get_transactions(&mut self) -> Result<Vec<Transaction>, AppError> {
        if self.transactions.is_none() {
            self.transactions = Some(self.repository.get_transactions_for_period(&self.budget_period.id).await?);
        }

        Ok(self.transactions.clone().unwrap())
    }

    async fn get_budget_categories(&mut self) -> Result<Vec<BudgetCategory>, AppError> {
        if self.budget_categories.is_none() {
            self.budget_categories = Some(self.repository.list_budget_categories().await?);
        }

        Ok(self.budget_categories.clone().unwrap())
    }

    async fn get_accounts(&mut self) -> Result<Vec<Account>, AppError> {
        if self.accounts.is_none() {
            self.accounts = Some(self.repository.list_accounts().await?);
        }

        Ok(self.accounts.clone().unwrap())
    }

    async fn get_all_transactions(&mut self) -> Result<Vec<Transaction>, AppError> {
        if self.all_transactions.is_none() {
            self.all_transactions = Some(self.repository.list_transactions().await?);
        }

        Ok(self.all_transactions.clone().unwrap())
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

        for account in self.get_accounts().await? {
            let mut current_date = self.budget_period.start_date;
            let mut balance = balance_on_date(Some(&self.budget_period.start_date), &account, &self.get_all_transactions().await?);

            while current_date <= Utc::now().date_naive() {
                balance = transactions
                    .iter()
                    .filter(|tx| account_involved(&account, tx) && tx.occurred_at == current_date)
                    .fold(balance, |acc, tx| add_transaction(acc, tx, &account));

                data.push(BudgetPerDayResponse {
                    account_name: account.name.clone(),
                    date: current_date.to_string(),
                    balance,
                });

                current_date = current_date.succ_opt().unwrap_or(self.budget_period.end_date);
            }
        }

        Ok(data)
    }

    pub async fn spent_per_category(&mut self) -> Result<Vec<SpentPerCategoryResponse>, AppError> {
        let transactions = self.get_transactions().await?;

        let mut data = self
            .get_budget_categories()
            .await?
            .into_iter()
            .map(|budget_category| {
                let amount_spent = transactions
                    .iter()
                    .filter(|tx| tx.category.id == budget_category.category.id)
                    .fold(0, |acc, tx| acc + tx.amount);
                SpentPerCategoryResponse {
                    category_name: budget_category.category.name,
                    budgeted_value: budget_category.budgeted_value as i32,
                    amount_spent,
                    percentage_spent: amount_spent * 10000 / (budget_category.budgeted_value as i32),
                }
            })
            .collect::<Vec<_>>();

        data.sort_by(|a, b| b.percentage_spent.cmp(&a.percentage_spent));

        Ok(data)
    }

    pub async fn monthly_burn_in(&mut self) -> Result<MonthlyBurnInResponse, AppError> {
        Ok(MonthlyBurnInResponse {
            total_budget: self.get_budget_categories().await?.iter().fold(0, |acc, bc| acc + bc.budgeted_value as i32),
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
