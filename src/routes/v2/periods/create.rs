use chrono::{Days, Months, NaiveDate};
use rocket::State;
use rocket::http::Status;
use rocket::post;
use rocket::serde::json::Json;
use sqlx::PgPool;
use validator::Validate;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::common::Date;
use crate::dto::period::{CreatePeriodRequest, DurationUnit, PeriodDuration, PeriodKind, PeriodResponse, PeriodStatus};
use crate::error::app_error::AppError;
use crate::models::budget_period::BudgetPeriodRequest;

/// Compute the end date from a start date and a duration specification.
fn compute_end_date(start: NaiveDate, duration: &PeriodDuration) -> Result<NaiveDate, AppError> {
    let n = duration.duration_units as u64;
    match duration.duration_unit {
        DurationUnit::Days => start
            .checked_add_days(Days::new(n))
            .ok_or_else(|| AppError::BadRequest("Duration overflow".to_string())),
        DurationUnit::Weeks => start
            .checked_add_days(Days::new(n * 7))
            .ok_or_else(|| AppError::BadRequest("Duration overflow".to_string())),
        DurationUnit::Months => start
            .checked_add_months(Months::new(n as u32))
            .ok_or_else(|| AppError::BadRequest("Duration overflow".to_string())),
    }
}

/// Compute the period length in days.
fn period_length(start: NaiveDate, end: NaiveDate) -> i64 {
    (end - start).num_days()
}

/// Determine the status of a period relative to today.
fn period_status(start: NaiveDate, end: NaiveDate) -> PeriodStatus {
    let today = chrono::Utc::now().date_naive();
    if today < start {
        PeriodStatus::Upcoming
    } else if today > end {
        PeriodStatus::Past
    } else {
        PeriodStatus::Active
    }
}

/// Compute remaining days (None if the period is past).
fn remaining_days(end: NaiveDate, status: PeriodStatus) -> Option<i64> {
    match status {
        PeriodStatus::Past => None,
        _ => {
            let today = chrono::Utc::now().date_naive();
            Some((end - today).num_days().max(0))
        }
    }
}

#[post("/", data = "<payload>")]
pub async fn create_period(pool: &State<PgPool>, user: CurrentUser, payload: Json<CreatePeriodRequest>) -> Result<(Status, Json<PeriodResponse>), AppError> {
    // Extract common fields and compute the end date + response kind.
    let (name, start_date, end_date, kind) = match &*payload {
        CreatePeriodRequest::Duration { start_date, name, duration } => {
            duration.validate()?;
            let end = compute_end_date(start_date.0, duration)?;
            let kind = PeriodKind::Duration {
                duration: PeriodDuration {
                    duration_units: duration.duration_units,
                    duration_unit: duration.duration_unit,
                },
            };
            (name.clone(), start_date.0, end, kind)
        }
        CreatePeriodRequest::ManualEndDate {
            start_date,
            name,
            manual_end_date,
        } => {
            if manual_end_date.0 <= start_date.0 {
                return Err(AppError::BadRequest("manualEndDate must be after startDate".to_string()));
            }
            let kind = PeriodKind::ManualEndDate {
                manual_end_date: Date(manual_end_date.0),
            };
            (name.clone(), start_date.0, manual_end_date.0, kind)
        }
    };

    if name.len() < 3 {
        return Err(AppError::BadRequest("name must be at least 3 characters".to_string()));
    }

    // Build the V1 request and delegate to the repository.
    let v1_request = BudgetPeriodRequest {
        name: name.clone(),
        start_date,
        end_date,
    };

    let repo = PostgresRepository { pool: pool.inner().clone() };
    let period_id = repo.create_budget_period(&v1_request, &user.id).await?;

    // Build the V2 response.
    let status = period_status(start_date, end_date);
    let response = PeriodResponse {
        id: period_id,
        start_date: Date(start_date),
        name,
        length: period_length(start_date, end_date),
        remaining_days: remaining_days(end_date, status),
        number_of_transactions: 0,
        percentage_of_target_used: None,
        status: Some(status),
        kind,
    };

    Ok((Status::Created, Json(response)))
}
