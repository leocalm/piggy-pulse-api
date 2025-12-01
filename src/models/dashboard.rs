use rocket::serde::Serialize;

#[derive(Serialize, Debug)]
pub struct BudgetPerDayResponse {
    pub account_name: String,
    pub date: String,
    pub balance: i32,
}

#[derive(Serialize, Debug)]
pub struct SpentPerCategoryResponse {
    pub category_name: String,
    pub budgeted_value: i32,
    pub amount_spent: i32,
}

#[derive(Serialize, Debug)]
pub struct MonthlyBurnInResponse {
    pub total_budget: i32,
    pub spent_budget: i32,
    pub current_day: i32,
    pub days_in_period: i32,
}

#[allow(dead_code)]
#[derive(Serialize, Debug)]
pub struct DashboardResponse {
    pub budget_per_day: Vec<BudgetPerDayResponse>,
    pub spent_per_category: Vec<SpentPerCategoryResponse>,
    pub monthly_burn_in: Vec<MonthlyBurnInResponse>,
}
