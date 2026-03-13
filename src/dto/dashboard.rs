#![allow(unused)]

use serde::Serialize;

// ===== CurrentPeriod =====

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CurrentPeriodResponse {
    pub spent: i64,
    pub target: i64,
    pub days_remaining: i64,
    pub days_in_period: i64,
    pub projected_spend: i64,
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
    pub periods_within_range: i64,
    pub periods_stability: Vec<bool>,
}
