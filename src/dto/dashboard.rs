use serde::Serialize;
use uuid::Uuid;

use crate::dto::common::Date;

// ===== CurrentPeriod =====

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CurrentPeriodResponse {
    pub spent: i64,
    pub target: i64,
    pub days_remaining: i64,
    pub days_in_period: i64,
    pub projected_spend: i64,
    pub daily_spend: Vec<i64>,
}

// ===== NetPosition =====

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct NetPositionResponse {
    pub total: i64,
    pub difference_this_period: i64,
    pub number_of_accounts: i64,
    pub liquid_amount: i64,
    pub protected_amount: i64,
    pub debt_amount: i64,
}

// ===== BudgetStability =====

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BudgetStabilityResponse {
    pub stability: i64,
    pub recent_stability: i64,
    pub periods_within_range: i64,
    pub periods_stability: Vec<bool>,
}

// ===== CashFlow =====

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CashFlowResponse {
    pub inflows: i64,
    pub outflows: i64,
    pub net: i64,
}

// ===== SpendingTrend =====

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SpendingTrendItem {
    pub period_id: Uuid,
    pub period_name: String,
    pub total_spent: i64,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SpendingTrendResponse {
    pub periods: Vec<SpendingTrendItem>,
    pub period_average: i64,
}

// ===== TopVendors =====

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TopVendorItem {
    pub vendor_id: Uuid,
    pub vendor_name: String,
    pub total_spent: i64,
    pub transaction_count: i64,
}

pub type TopVendorsResponse = Vec<TopVendorItem>;

// ===== Uncategorized =====

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UncategorizedTransaction {
    pub id: Uuid,
    pub amount: i64,
    pub date: Date,
    pub description: String,
    pub from_account_id: Uuid,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UncategorizedResponse {
    pub count: i64,
    pub transactions: Vec<UncategorizedTransaction>,
}

// ===== FixedCategories =====

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub enum FixedCategoryStatus {
    Paid,
    Partial,
    Pending,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FixedCategoryItem {
    pub category_id: Uuid,
    pub category_name: String,
    pub category_icon: String,
    pub status: FixedCategoryStatus,
    pub spent: i64,
    pub budgeted: i64,
}

pub type FixedCategoriesResponse = Vec<FixedCategoryItem>;

// ===== NetPositionHistory =====

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct NetPositionHistoryPoint {
    pub date: String,
    pub total: i64,
    pub liquid_amount: i64,
    pub protected_amount: i64,
    pub debt_amount: i64,
}

pub type NetPositionHistoryResponse = Vec<NetPositionHistoryPoint>;

// ===== CurrentPeriodHistory =====

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrentPeriodHistoryPoint {
    pub date: String,
    pub cumulative_spent: i64,
    pub daily_spent: i64,
}

pub type CurrentPeriodHistoryResponse = Vec<CurrentPeriodHistoryPoint>;

// ===== Subscriptions =====

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub enum SubscriptionDisplayStatus {
    Charged,
    Today,
    Upcoming,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub enum SubscriptionBillingCycle {
    Monthly,
    Quarterly,
    Yearly,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionDashboardItem {
    pub id: Uuid,
    pub name: String,
    pub billing_amount: i64,
    pub billing_cycle: SubscriptionBillingCycle,
    pub next_charge_date: String,
    pub display_status: SubscriptionDisplayStatus,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionsDashboardResponse {
    pub active_count: i64,
    pub monthly_total: i64,
    pub yearly_total: i64,
    pub subscriptions: Vec<SubscriptionDashboardItem>,
}
