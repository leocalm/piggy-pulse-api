use chrono::{Days, Months, NaiveDate};
use uuid::Uuid;

use crate::database::budget_period::{V2PeriodRow, V2ScheduleParams};
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::common::{Date, PaginatedResponse};
use crate::dto::period::{
    CreatePeriodRequest, CreatePeriodScheduleRequest, DurationUnit, PeriodDuration, PeriodGap, PeriodGapsResponse, PeriodKind, PeriodResponse,
    PeriodScheduleResponse, PeriodStatus, ScheduleKind, UpdatePeriodRequest, UpdatePeriodScheduleRequest, WeekendPolicy,
};
use crate::error::app_error::AppError;
use crate::models::budget_period::{BudgetPeriodRequest, PeriodSchedule};

pub struct PeriodService<'a> {
    repository: &'a PostgresRepository,
}

impl<'a> PeriodService<'a> {
    pub fn new(repository: &'a PostgresRepository) -> Self {
        PeriodService { repository }
    }

    // ===== Period CRUD =====

    pub async fn create_period(&self, request: &CreatePeriodRequest, user_id: &Uuid) -> Result<PeriodResponse, AppError> {
        let (name, start_date, end_date, kind) = extract_period_fields(request)?;

        if name.len() < 3 {
            return Err(AppError::BadRequest("name must be at least 3 characters".to_string()));
        }

        let v1_request = BudgetPeriodRequest {
            name: name.clone(),
            start_date,
            end_date,
        };
        let period_id = self.repository.create_budget_period(&v1_request, user_id).await?;

        let status = compute_status(start_date, end_date);
        Ok(PeriodResponse {
            id: period_id,
            start_date: Date(start_date),
            name,
            length: compute_length(start_date, end_date),
            remaining_days: compute_remaining_days(end_date, status),
            number_of_transactions: 0, // just created, no transactions yet
            percentage_of_target_used: None,
            status: Some(status),
            kind,
        })
    }

    pub async fn get_period(&self, id: &Uuid, user_id: &Uuid) -> Result<PeriodResponse, AppError> {
        let row = self
            .repository
            .get_budget_period_v2(id, user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Period not found".to_string()))?;

        Ok(row_to_response(&row))
    }

    pub async fn list_periods(&self, cursor: Option<Uuid>, limit: i64, user_id: &Uuid) -> Result<PaginatedResponse<PeriodResponse>, AppError> {
        let (mut rows, total_count) = self.repository.list_budget_periods_v2(cursor, limit, user_id).await?;

        let has_more = rows.len() as i64 > limit;
        if has_more {
            rows.truncate(limit as usize);
        }
        let next_cursor = if has_more { rows.last().map(|r| r.id.to_string()) } else { None };

        let data: Vec<PeriodResponse> = rows.iter().map(row_to_response).collect();

        Ok(PaginatedResponse {
            data,
            total_count,
            has_more,
            next_cursor,
        })
    }

    pub async fn update_period(&self, id: &Uuid, request: &UpdatePeriodRequest, user_id: &Uuid) -> Result<PeriodResponse, AppError> {
        let (name, start_date, end_date, _kind) = extract_period_fields(request)?;

        if name.len() < 3 {
            return Err(AppError::BadRequest("name must be at least 3 characters".to_string()));
        }

        let v1_request = BudgetPeriodRequest { name, start_date, end_date };

        // This will raise RowNotFound -> 404 if it doesn't exist
        let _updated = self.repository.update_budget_period(id, &v1_request, user_id).await?;

        // Re-fetch the period with transaction count
        let row = self
            .repository
            .get_budget_period_v2(id, user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Period not found".to_string()))?;

        Ok(row_to_response(&row))
    }

    pub async fn delete_period(&self, id: &Uuid, user_id: &Uuid) -> Result<(), AppError> {
        // Check existence first for 404
        self.repository
            .get_budget_period_v2(id, user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Period not found".to_string()))?;

        self.repository.delete_budget_period(id, user_id).await?;
        Ok(())
    }

    // ===== Schedule CRUD =====

    pub async fn get_schedule(&self, user_id: &Uuid) -> Result<PeriodScheduleResponse, AppError> {
        let schedule = self.repository.get_period_schedule(user_id).await?;
        Ok(schedule_to_response(&schedule))
    }

    pub async fn create_schedule(&self, request: &CreatePeriodScheduleRequest, user_id: &Uuid) -> Result<PeriodScheduleResponse, AppError> {
        let params = schedule_request_to_params(request)?;
        let schedule = self.repository.create_period_schedule_v2(&params, user_id).await?;
        Ok(schedule_to_response(&schedule))
    }

    pub async fn update_schedule(&self, request: &UpdatePeriodScheduleRequest, user_id: &Uuid) -> Result<PeriodScheduleResponse, AppError> {
        let params = schedule_request_to_params(request)?;
        let schedule = self.repository.update_period_schedule_v2(&params, user_id).await?;
        Ok(schedule_to_response(&schedule))
    }

    pub async fn delete_schedule(&self, user_id: &Uuid) -> Result<(), AppError> {
        let deleted = self.repository.delete_period_schedule_v2(user_id).await?;
        if !deleted {
            return Err(AppError::NotFound("Schedule not found".to_string()));
        }
        Ok(())
    }

    pub async fn get_gaps(&self, user_id: &Uuid) -> Result<PeriodGapsResponse, AppError> {
        let ranges = self.repository.list_period_date_ranges_v2(user_id).await?;

        let mut gaps = Vec::new();

        for window in ranges.windows(2) {
            let (_, end_a) = window[0];
            let (start_b, _) = window[1];

            // gap exists if there is at least one day between the end of period A and the start of period B
            if let Some(gap_start) = end_a.checked_add_days(Days::new(1))
                && gap_start < start_b
                && let Some(gap_end) = start_b.checked_sub_days(Days::new(1))
            {
                gaps.push(PeriodGap {
                    start_date: Date(gap_start),
                    end_date: Date(gap_end),
                });
            }
        }

        Ok(gaps)
    }
}

// ===== Helpers =====

fn extract_period_fields(request: &CreatePeriodRequest) -> Result<(String, NaiveDate, NaiveDate, PeriodKind), AppError> {
    match request {
        CreatePeriodRequest::Duration { start_date, name, duration } => {
            use validator::Validate;
            duration.validate()?;
            let end = compute_end_date(start_date.0, duration)?;
            let kind = PeriodKind::Duration {
                duration: PeriodDuration {
                    duration_units: duration.duration_units,
                    duration_unit: duration.duration_unit,
                },
            };
            Ok((name.clone(), start_date.0, end, kind))
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
            Ok((name.clone(), start_date.0, manual_end_date.0, kind))
        }
    }
}

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

fn compute_length(start: NaiveDate, end: NaiveDate) -> i64 {
    (end - start).num_days()
}

fn compute_status(start: NaiveDate, end: NaiveDate) -> PeriodStatus {
    let today = chrono::Utc::now().date_naive();
    if today < start {
        PeriodStatus::Upcoming
    } else if today > end {
        PeriodStatus::Past
    } else {
        PeriodStatus::Active
    }
}

fn compute_remaining_days(end: NaiveDate, status: PeriodStatus) -> Option<i64> {
    match status {
        PeriodStatus::Past => None,
        _ => {
            let today = chrono::Utc::now().date_naive();
            Some((end - today).num_days().max(0))
        }
    }
}

/// Convert a database row into a V2 PeriodResponse, inferring periodType from
/// whether an explicit duration was stored. Since the DB only stores start_date
/// and end_date, we always return ManualEndDate discriminator.
fn row_to_response(row: &V2PeriodRow) -> PeriodResponse {
    let status = compute_status(row.start_date, row.end_date);
    PeriodResponse {
        id: row.id,
        start_date: Date(row.start_date),
        name: row.name.clone(),
        length: compute_length(row.start_date, row.end_date),
        remaining_days: compute_remaining_days(row.end_date, status),
        number_of_transactions: row.transaction_count.unwrap_or(0),
        percentage_of_target_used: None,
        status: Some(status),
        kind: PeriodKind::ManualEndDate {
            manual_end_date: Date(row.end_date),
        },
    }
}

fn schedule_to_response(schedule: &PeriodSchedule) -> PeriodScheduleResponse {
    let kind = if schedule.schedule_type == "automatic" {
        ScheduleKind::Automatic {
            start_day_of_the_month: schedule.start_day.unwrap_or(1) as i64,
            period_duration: schedule.duration_value.unwrap_or(1) as i64,
            generate_ahead: schedule.generate_ahead.unwrap_or(1) as i64,
            duration_unit: schedule
                .duration_unit
                .map(|du| match du {
                    crate::models::budget_period::DurationUnit::Days => DurationUnit::Days,
                    crate::models::budget_period::DurationUnit::Weeks => DurationUnit::Weeks,
                    crate::models::budget_period::DurationUnit::Months => DurationUnit::Months,
                })
                .unwrap_or(DurationUnit::Days),
            saturday_policy: schedule
                .saturday_adjustment
                .map(|wa| match wa {
                    crate::models::budget_period::WeekendAdjustment::Keep => crate::dto::period::WeekendPolicy::Keep,
                    crate::models::budget_period::WeekendAdjustment::Friday => crate::dto::period::WeekendPolicy::Friday,
                    crate::models::budget_period::WeekendAdjustment::Monday => crate::dto::period::WeekendPolicy::Monday,
                })
                .unwrap_or(crate::dto::period::WeekendPolicy::Keep),
            sunday_policy: schedule
                .sunday_adjustment
                .map(|wa| match wa {
                    crate::models::budget_period::WeekendAdjustment::Keep => crate::dto::period::WeekendPolicy::Keep,
                    crate::models::budget_period::WeekendAdjustment::Friday => crate::dto::period::WeekendPolicy::Friday,
                    crate::models::budget_period::WeekendAdjustment::Monday => crate::dto::period::WeekendPolicy::Monday,
                })
                .unwrap_or(crate::dto::period::WeekendPolicy::Keep),
            name_pattern: schedule.name_pattern.clone().unwrap_or_default(),
        }
    } else {
        ScheduleKind::Manual
    };

    PeriodScheduleResponse {
        id: schedule.id,
        schedule: kind,
    }
}

fn schedule_request_to_params(request: &CreatePeriodScheduleRequest) -> Result<V2ScheduleParams<'_>, AppError> {
    match request {
        CreatePeriodScheduleRequest::Manual => Ok(V2ScheduleParams {
            schedule_type: "manual",
            start_day: None,
            duration_value: None,
            duration_unit: None,
            saturday_adjustment: None,
            sunday_adjustment: None,
            name_pattern: None,
            generate_ahead: None,
        }),
        CreatePeriodScheduleRequest::Automatic {
            start_day_of_the_month,
            period_duration,
            generate_ahead,
            duration_unit,
            saturday_policy,
            sunday_policy,
            name_pattern,
        } => {
            validate_schedule_fields(*start_day_of_the_month, *period_duration, *generate_ahead)?;
            Ok(V2ScheduleParams {
                schedule_type: "automatic",
                start_day: Some(*start_day_of_the_month as i32),
                duration_value: Some(*period_duration as i32),
                duration_unit: Some(duration_unit_to_str(duration_unit)),
                saturday_adjustment: Some(weekend_policy_to_str(saturday_policy)),
                sunday_adjustment: Some(weekend_policy_to_str(sunday_policy)),
                name_pattern: Some(name_pattern.as_str()),
                generate_ahead: Some(*generate_ahead as i32),
            })
        }
    }
}

fn duration_unit_to_str(du: &DurationUnit) -> &'static str {
    match du {
        DurationUnit::Days => "days",
        DurationUnit::Weeks => "weeks",
        DurationUnit::Months => "months",
    }
}

fn weekend_policy_to_str(wp: &WeekendPolicy) -> &'static str {
    match wp {
        WeekendPolicy::Keep => "keep",
        WeekendPolicy::Monday => "monday",
        WeekendPolicy::Friday => "friday",
    }
}

fn validate_schedule_fields(start_day: i64, period_duration: i64, generate_ahead: i64) -> Result<(), AppError> {
    if !(1..=31).contains(&start_day) {
        return Err(AppError::BadRequest("startDayOfTheMonth must be between 1 and 31".to_string()));
    }
    if !(1..=i32::MAX as i64).contains(&period_duration) {
        return Err(AppError::BadRequest("periodDuration must be between 1 and 2147483647".to_string()));
    }
    if !(1..=i32::MAX as i64).contains(&generate_ahead) {
        return Err(AppError::BadRequest("generateAhead must be between 1 and 2147483647".to_string()));
    }
    Ok(())
}
