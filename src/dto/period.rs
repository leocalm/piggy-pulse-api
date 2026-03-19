#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::dto::common::{Date, PaginatedResponse};

// ===== Enums =====

#[derive(Serialize, Deserialize, Debug, Copy, Clone, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DurationUnit {
    Days,
    Weeks,
    Months,
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PeriodStatus {
    Active,
    Upcoming,
    Past,
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum WeekendPolicy {
    Keep,
    Monday,
    Friday,
}

// ===== PeriodDuration =====

#[derive(Serialize, Deserialize, Debug, Validate)]
#[serde(rename_all = "camelCase")]
pub struct PeriodDuration {
    #[validate(range(min = 1))]
    pub duration_units: i64,
    pub duration_unit: DurationUnit,
}

// ===== PeriodKind (response discriminator, flattened into PeriodResponse) =====

#[derive(Serialize, Debug)]
#[serde(tag = "periodType", rename_all = "camelCase")]
pub enum PeriodKind {
    Duration {
        duration: PeriodDuration,
    },
    ManualEndDate {
        #[serde(rename = "manualEndDate")]
        manual_end_date: Date,
    },
}

// ===== PeriodResponse =====

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PeriodResponse {
    pub id: Uuid,
    pub start_date: Date,
    pub name: String,
    pub length: i64,
    pub remaining_days: Option<i64>,
    pub number_of_transactions: i64,
    pub percentage_of_target_used: Option<i64>,
    pub status: Option<PeriodStatus>,
    #[serde(flatten)]
    pub kind: PeriodKind,
}

// ===== PeriodListResponse =====

pub type PeriodListResponse = PaginatedResponse<PeriodResponse>;

// ===== CreatePeriodRequest / UpdatePeriodRequest =====

/// Top-level internally-tagged enum avoids the serde flatten+tag limitation on the Deserialize path.
/// validator 0.20 does not support #[derive(Validate)] on enums; field-level validation on
/// PeriodDuration is enforced via its own Validate impl when called explicitly by the route layer.
#[derive(Deserialize, Debug)]
#[serde(tag = "periodType")]
pub enum CreatePeriodRequest {
    Duration {
        #[serde(rename = "startDate")]
        start_date: Date,
        name: String,
        duration: PeriodDuration,
    },
    ManualEndDate {
        #[serde(rename = "startDate")]
        start_date: Date,
        name: String,
        #[serde(rename = "manualEndDate")]
        manual_end_date: Date,
    },
}

pub type UpdatePeriodRequest = CreatePeriodRequest;

// ===== ScheduleKind (response discriminator, flattened into PeriodScheduleResponse) =====

#[derive(Serialize, Debug)]
#[serde(tag = "scheduleType", rename_all = "camelCase")]
pub enum ScheduleKind {
    Manual,
    Automatic {
        #[serde(rename = "startDayOfTheMonth")]
        start_day_of_the_month: i64,
        #[serde(rename = "periodDuration")]
        period_duration: i64,
        #[serde(rename = "generateAhead")]
        generate_ahead: i64,
        #[serde(rename = "durationUnit")]
        duration_unit: DurationUnit,
        #[serde(rename = "saturdayPolicy")]
        saturday_policy: WeekendPolicy,
        #[serde(rename = "sundayPolicy")]
        sunday_policy: WeekendPolicy,
        #[serde(rename = "namePattern")]
        name_pattern: String,
    },
}

// ===== PeriodScheduleResponse =====

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PeriodScheduleResponse {
    pub id: Uuid,
    #[serde(flatten)]
    pub schedule: ScheduleKind,
}

// ===== CreatePeriodScheduleRequest / UpdatePeriodScheduleRequest =====

/// Top-level internally-tagged enum avoids the serde flatten+tag limitation on the Deserialize path.
/// validator 0.20 does not support #[derive(Validate)] on enums; range validation for schedule
/// fields must be enforced explicitly by the route layer.
#[derive(Deserialize, Debug)]
#[serde(tag = "scheduleType", rename_all = "camelCase")]
pub enum CreatePeriodScheduleRequest {
    Manual,
    Automatic {
        #[serde(rename = "startDayOfTheMonth")]
        start_day_of_the_month: i64,
        #[serde(rename = "periodDuration")]
        period_duration: i64,
        #[serde(rename = "generateAhead")]
        generate_ahead: i64,
        #[serde(rename = "durationUnit")]
        duration_unit: DurationUnit,
        #[serde(rename = "saturdayPolicy")]
        saturday_policy: WeekendPolicy,
        #[serde(rename = "sundayPolicy")]
        sunday_policy: WeekendPolicy,
        #[serde(rename = "namePattern")]
        name_pattern: String,
    },
}

pub type UpdatePeriodScheduleRequest = CreatePeriodScheduleRequest;
