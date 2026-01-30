use chrono::{DateTime, NaiveDate, Utc};
use rocket::serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Debug, Default)]
pub struct BudgetPeriod {
    pub id: Uuid,
    pub name: String,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub created_at: DateTime<Utc>,
}

#[derive(Deserialize, Debug)]
pub struct BudgetPeriodRequest {
    pub name: String,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
}

#[derive(Serialize, Debug)]
pub struct BudgetPeriodResponse {
    pub id: Uuid,
    pub name: String,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
}

impl From<&BudgetPeriod> for BudgetPeriodResponse {
    fn from(budget_period: &BudgetPeriod) -> Self {
        Self {
            id: budget_period.id,
            name: budget_period.name.clone(),
            start_date: budget_period.start_date,
            end_date: budget_period.end_date,
        }
    }
}
