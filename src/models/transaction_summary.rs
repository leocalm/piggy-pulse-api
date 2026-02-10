use rocket::serde::Serialize;
use schemars::JsonSchema;

#[derive(Serialize, Debug)]
pub struct TransactionSummary {
    pub total_income: i32,
    pub total_expense: i32,
    pub net_difference: i32,
}

#[derive(Serialize, Debug, JsonSchema)]
pub struct TransactionSummaryResponse {
    pub total_income: i32,
    pub total_expense: i32,
    pub net_difference: i32,
}

impl From<&TransactionSummary> for TransactionSummaryResponse {
    fn from(summary: &TransactionSummary) -> Self {
        Self {
            total_income: summary.total_income,
            total_expense: summary.total_expense,
            net_difference: summary.net_difference,
        }
    }
}
