use crate::models::transaction::TransactionResponse;
use chrono::NaiveDate;
use rocket::serde::Serialize;
use schemars::JsonSchema;

#[derive(Serialize, Debug, JsonSchema)]
pub struct BudgetPerDayResponse {
    pub account_name: String,
    pub date: String,
    pub balance: i32,
}

#[derive(Serialize, Debug, Ord, PartialOrd, Eq, PartialEq, Clone, JsonSchema)]
pub struct SpentPerCategoryResponse {
    pub category_name: String,
    pub budgeted_value: i32,
    pub amount_spent: i32,
    pub percentage_spent: i32,
}

#[derive(Serialize, Debug, sqlx::FromRow, JsonSchema)]
pub struct MonthlyBurnInResponse {
    pub total_budget: i32,
    pub spent_budget: i32,
    pub current_day: i32,
    pub days_in_period: i32,
}

#[derive(Serialize, Debug, JsonSchema)]
pub struct MonthProgressResponse {
    pub current_date: NaiveDate,
    pub days_in_period: u32,
    pub remaining_days: u32,
    pub days_passed_percentage: u32,
}

#[derive(Serialize, Debug, JsonSchema)]
pub struct DashboardResponse {
    pub budget_per_day: Vec<BudgetPerDayResponse>,
    pub spent_per_category: Vec<SpentPerCategoryResponse>,
    pub monthly_burn_in: MonthlyBurnInResponse,
    pub month_progress: MonthProgressResponse,
    pub recent_transactions: Vec<TransactionResponse>,
    pub total_asset: i32,
}
