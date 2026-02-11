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

#[derive(Debug, Clone)]
pub struct BudgetPeriodWithMetrics {
    pub period: BudgetPeriod,
    pub transaction_count: i64,
    pub budget_used_percentage: f64,
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

#[derive(Serialize, Debug, JsonSchema)]
pub struct BudgetPeriodResponse {
    pub id: Uuid,
    pub name: String,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub is_auto_generated: bool,
    pub transaction_count: i64,
    pub budget_used_percentage: f64,
}

impl From<&BudgetPeriod> for BudgetPeriodResponse {
    fn from(budget_period: &BudgetPeriod) -> Self {
        Self {
            id: budget_period.id,
            name: budget_period.name.clone(),
            start_date: budget_period.start_date,
            end_date: budget_period.end_date,
            is_auto_generated: budget_period.is_auto_generated,
            transaction_count: 0,
            budget_used_percentage: 0.0,
        }
    }
}

impl From<&BudgetPeriodWithMetrics> for BudgetPeriodResponse {
    fn from(period_with_metrics: &BudgetPeriodWithMetrics) -> Self {
        Self {
            id: period_with_metrics.period.id,
            name: period_with_metrics.period.name.clone(),
            start_date: period_with_metrics.period.start_date,
            end_date: period_with_metrics.period.end_date,
            is_auto_generated: period_with_metrics.period.is_auto_generated,
            transaction_count: period_with_metrics.transaction_count,
            budget_used_percentage: period_with_metrics.budget_used_percentage,
        }
    }
}
// ===== Period Schedule Models =====

#[derive(Serialize, Debug, Clone, sqlx::FromRow)]
pub struct PeriodSchedule {
    pub id: Uuid,
    pub user_id: Uuid,
    pub start_day: i32,
    pub duration_value: i32,
    pub duration_unit: DurationUnit,
    pub saturday_adjustment: WeekendAdjustment,
    pub sunday_adjustment: WeekendAdjustment,
    pub name_pattern: String,
    pub generate_ahead: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Deserialize, Debug, Validate, JsonSchema)]
pub struct PeriodScheduleRequest {
    #[validate(range(min = 1, max = 31))]
    pub start_day: i32,
    #[validate(range(min = 1))]
    pub duration_value: i32,
    pub duration_unit: DurationUnit,
    pub saturday_adjustment: WeekendAdjustment,
    pub sunday_adjustment: WeekendAdjustment,
    #[validate(length(min = 1))]
    pub name_pattern: String,
    #[validate(range(min = 0))]
    pub generate_ahead: i32,
}

#[derive(Serialize, Debug, JsonSchema)]
pub struct PeriodScheduleResponse {
    pub id: Uuid,
    pub start_day: i32,
    pub duration_value: i32,
    pub duration_unit: DurationUnit,
    pub saturday_adjustment: WeekendAdjustment,
    pub sunday_adjustment: WeekendAdjustment,
    pub name_pattern: String,
    pub generate_ahead: i32,
}

impl From<&PeriodSchedule> for PeriodScheduleResponse {
    fn from(schedule: &PeriodSchedule) -> Self {
        Self {
            id: schedule.id,
            start_day: schedule.start_day,
            duration_value: schedule.duration_value,
            duration_unit: schedule.duration_unit,
            saturday_adjustment: schedule.saturday_adjustment,
            sunday_adjustment: schedule.sunday_adjustment,
            name_pattern: schedule.name_pattern.clone(),
            generate_ahead: schedule.generate_ahead,
        }
    }
}

// ===== Gap Detection Models =====

#[derive(Serialize, Debug, Clone, JsonSchema)]
pub struct UnassignedTransaction {
    pub id: Uuid,
    pub occurred_at: NaiveDate,
    pub description: String,
    pub amount: i64,
}

#[derive(Serialize, Debug, JsonSchema)]
pub struct GapsResponse {
    pub unassigned_count: i64,
    pub transactions: Vec<UnassignedTransaction>,
}

#[derive(Serialize, Debug)]
pub struct AutoPeriodGenerationResponse {
    pub users_processed: i64,
    pub periods_created: i64,
}
