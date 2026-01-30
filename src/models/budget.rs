use chrono::{DateTime, Utc};
use rocket::serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Serialize, Debug)]
pub struct Budget {
    pub id: Uuid,
    pub name: String,
    pub start_day: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Deserialize, Debug, Validate)]
pub struct BudgetRequest {
    #[validate(length(min = 3))]
    pub name: String,
    #[validate(range(min = 0, max = 31))]
    pub start_day: i32,
}

#[derive(Serialize, Debug)]
pub struct BudgetResponse {
    pub id: Uuid,
    pub name: String,
    pub start_day: i32,
}

impl From<&Budget> for BudgetResponse {
    fn from(budget: &Budget) -> Self {
        Self {
            id: budget.id,
            name: budget.name.clone(),
            start_day: budget.start_day,
        }
    }
}
