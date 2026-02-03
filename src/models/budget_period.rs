use chrono::{DateTime, NaiveDate, Utc};
use rocket::serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::{Validate, ValidationError};

#[derive(Serialize, Debug, Default, sqlx::FromRow)]
pub struct BudgetPeriod {
    pub id: Uuid,
    pub name: String,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub created_at: DateTime<Utc>,
}

#[derive(Deserialize, Debug, Validate)]
#[validate(schema(function = "validate_date_range"))]
pub struct BudgetPeriodRequest {
    #[validate(length(min = 3))]
    pub name: String,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
}

fn validate_date_range(request: &BudgetPeriodRequest) -> Result<(), ValidationError> {
    if request.start_date >= request.end_date {
        return Err(ValidationError::new("start_date_must_be_before_end_date"));
    }
    Ok(())
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
