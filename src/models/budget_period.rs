use chrono::{DateTime, NaiveDate, Utc};
use rocket::serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use uuid::Uuid;
use validator::{Validate, ValidationError};

// ===== Enums =====

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, sqlx::Type)]
#[sqlx(type_name = "text", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum DurationUnit {
    Days,
    Weeks,
    Months,
}

impl std::fmt::Display for DurationUnit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DurationUnit::Days => write!(f, "days"),
            DurationUnit::Weeks => write!(f, "weeks"),
            DurationUnit::Months => write!(f, "months"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, sqlx::Type)]
#[sqlx(type_name = "text", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum WeekendAdjustment {
    Keep,
    Friday,
    Monday,
}

impl std::fmt::Display for WeekendAdjustment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WeekendAdjustment::Keep => write!(f, "keep"),
            WeekendAdjustment::Friday => write!(f, "friday"),
            WeekendAdjustment::Monday => write!(f, "monday"),
        }
    }
}

// ===== Budget Period Models =====

#[derive(Serialize, Debug, Clone, Default, sqlx::FromRow)]
pub struct BudgetPeriod {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub is_auto_generated: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Deserialize, Debug, Validate, JsonSchema)]
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

// ===== Period Schedule Models =====

#[derive(Serialize, Debug, Clone, sqlx::FromRow)]
pub struct PeriodSchedule {
    pub id: Uuid,
    pub user_id: Uuid,
    pub schedule_type: String,
    pub start_day: Option<i32>,
    pub duration_value: Option<i32>,
    pub duration_unit: Option<DurationUnit>,
    pub saturday_adjustment: Option<WeekendAdjustment>,
    pub sunday_adjustment: Option<WeekendAdjustment>,
    pub name_pattern: Option<String>,
    pub generate_ahead: Option<i32>,
    pub recurrence_method: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Serialize, Debug)]
pub struct AutoPeriodGenerationResponse {
    pub users_processed: i64,
    pub periods_created: i64,
}
