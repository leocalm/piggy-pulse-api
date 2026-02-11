use crate::database::postgres_repository::{PostgresRepository, is_unique_violation};
use crate::error::app_error::AppError;
use crate::models::budget_period::{
    AutoPeriodGenerationResponse, BudgetPeriod, BudgetPeriodRequest, BudgetPeriodWithMetrics, DurationUnit, GapsResponse, PeriodSchedule,
    PeriodScheduleRequest, UnassignedTransaction, WeekendAdjustment,
};
use crate::models::pagination::CursorParams;
use chrono::{Datelike, Days, Months, NaiveDate, Weekday};
use uuid::Uuid;

impl PostgresRepository {
    pub async fn create_budget_period(&self, request: &BudgetPeriodRequest, user_id: &Uuid) -> Result<Uuid, AppError> {
        let name_exists: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS (
                SELECT 1
                FROM budget_period
                WHERE user_id = $1 AND name = $2
            )
            "#,
        )
        .bind(user_id)
        .bind(&request.name)
        .fetch_one(&self.pool)
        .await?;

        if name_exists {
            return Err(AppError::BadRequest("Budget period name already exists".to_string()));
        }

        #[derive(sqlx::FromRow)]
        struct IdRow {
            id: Uuid,
        }

        let row = sqlx::query_as::<_, IdRow>(
            r#"
            INSERT INTO budget_period (user_id, name, start_date, end_date)
            VALUES ($1, $2, $3, $4)
            RETURNING id
            "#,
        )
        .bind(user_id)
        .bind(&request.name)
        .bind(request.start_date)
        .bind(request.end_date)
        .fetch_one(&self.pool)
        .await;

        let row = match row {
            Ok(row) => row,
            Err(err) if is_unique_violation(&err) => {
                return Err(AppError::BadRequest("Budget period name already exists".to_string()));
            }
            Err(err) => return Err(err.into()),
        };

        Ok(row.id)
    }

    pub async fn get_budget_period(&self, budget_period_id: &Uuid, user_id: &Uuid) -> Result<BudgetPeriod, AppError> {
        let budget_period = sqlx::query_as::<_, BudgetPeriod>(
            r#"
            SELECT id, user_id, name, start_date, end_date, is_auto_generated, created_at
            FROM budget_period
            WHERE id = $1 AND user_id = $2
            "#,
        )
        .bind(budget_period_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(budget_period)
    }

    pub async fn update_budget_period(&self, id: &Uuid, request: &BudgetPeriodRequest, user_id: &Uuid) -> Result<BudgetPeriod, AppError> {
        let name_exists: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS (
                SELECT 1
                FROM budget_period
                WHERE user_id = $1 AND name = $2 AND id <> $3
            )
            "#,
        )
        .bind(user_id)
        .bind(&request.name)
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        if name_exists {
            return Err(AppError::BadRequest("Budget period name already exists".to_string()));
        }

        let budget_period = sqlx::query_as::<_, BudgetPeriod>(
            r#"
            UPDATE budget_period
            SET name = $1, start_date = $2, end_date = $3
            WHERE id = $4 AND user_id = $5
            RETURNING id, user_id, name, start_date, end_date, is_auto_generated, created_at
            "#,
        )
        .bind(&request.name)
        .bind(request.start_date)
        .bind(request.end_date)
        .bind(id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await;

        let budget_period = match budget_period {
            Ok(budget_period) => budget_period,
            Err(err) if is_unique_violation(&err) => {
                return Err(AppError::BadRequest("Budget period name already exists".to_string()));
            }
            Err(err) => return Err(err.into()),
        };

        Ok(budget_period)
    }

    pub async fn delete_budget_period(&self, id: &Uuid, user_id: &Uuid) -> Result<(), AppError> {
        sqlx::query("DELETE FROM budget_period WHERE id = $1 AND user_id = $2")
            .bind(id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    // ===== Period Schedule Methods =====

    pub async fn create_period_schedule(&self, request: &PeriodScheduleRequest, user_id: &Uuid) -> Result<PeriodSchedule, AppError> {
        let schedule = sqlx::query_as::<_, PeriodSchedule>(
            r#"
            INSERT INTO period_schedule (
                user_id, start_day, duration_value, duration_unit,
                saturday_adjustment, sunday_adjustment, name_pattern, generate_ahead
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id, user_id, start_day, duration_value, duration_unit,
                      saturday_adjustment, sunday_adjustment, name_pattern,
                      generate_ahead, created_at, updated_at
            "#,
        )
        .bind(user_id)
        .bind(request.start_day)
        .bind(request.duration_value)
        .bind(request.duration_unit)
        .bind(request.saturday_adjustment)
        .bind(request.sunday_adjustment)
        .bind(&request.name_pattern)
        .bind(request.generate_ahead)
        .fetch_one(&self.pool)
        .await?;

        Ok(schedule)
    }

    pub async fn get_period_schedule(&self, user_id: &Uuid) -> Result<PeriodSchedule, AppError> {
        let schedule = sqlx::query_as::<_, PeriodSchedule>(
            r#"
            SELECT id, user_id, start_day, duration_value, duration_unit,
                   saturday_adjustment, sunday_adjustment, name_pattern,
                   generate_ahead, created_at, updated_at
            FROM period_schedule
            WHERE user_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(schedule)
    }

    pub async fn update_period_schedule(&self, request: &PeriodScheduleRequest, user_id: &Uuid) -> Result<PeriodSchedule, AppError> {
        let schedule = sqlx::query_as::<_, PeriodSchedule>(
            r#"
            UPDATE period_schedule
            SET start_day = $1,
                duration_value = $2,
                duration_unit = $3,
                saturday_adjustment = $4,
                sunday_adjustment = $5,
                name_pattern = $6,
                generate_ahead = $7,
                updated_at = now()
            WHERE user_id = $8
            RETURNING id, user_id, start_day, duration_value, duration_unit,
                      saturday_adjustment, sunday_adjustment, name_pattern,
                      generate_ahead, created_at, updated_at
            "#,
        )
        .bind(request.start_day)
        .bind(request.duration_value)
        .bind(request.duration_unit)
        .bind(request.saturday_adjustment)
        .bind(request.sunday_adjustment)
        .bind(&request.name_pattern)
        .bind(request.generate_ahead)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(schedule)
    }

    pub async fn delete_period_schedule(&self, user_id: &Uuid) -> Result<(), AppError> {
        sqlx::query("DELETE FROM period_schedule WHERE user_id = $1")
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn generate_automatic_budget_periods(&self) -> Result<AutoPeriodGenerationResponse, AppError> {
        #[derive(sqlx::FromRow)]
        struct ScheduleRow {
            user_id: Uuid,
            start_day: i32,
            duration_value: i32,
            duration_unit: DurationUnit,
            saturday_adjustment: WeekendAdjustment,
            sunday_adjustment: WeekendAdjustment,
            name_pattern: String,
            generate_ahead: i32,
        }

        let schedules = sqlx::query_as::<_, ScheduleRow>(
            r#"
            SELECT user_id, start_day, duration_value, duration_unit,
                   saturday_adjustment, sunday_adjustment, name_pattern, generate_ahead
            FROM period_schedule
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let today = chrono::Utc::now().date_naive();
        let mut periods_created = 0_i64;

        for schedule in &schedules {
            let existing_future_count: i64 = sqlx::query_scalar(
                r#"
                SELECT COUNT(*)
                FROM budget_period
                WHERE user_id = $1
                  AND end_date >= $2
                "#,
            )
            .bind(schedule.user_id)
            .bind(today)
            .fetch_one(&self.pool)
            .await?;

            let missing = (schedule.generate_ahead as i64) - existing_future_count;
            if missing <= 0 {
                continue;
            }

            let max_end_date: Option<NaiveDate> = sqlx::query_scalar(
                r#"
                SELECT MAX(end_date)
                FROM budget_period
                WHERE user_id = $1
                "#,
            )
            .bind(schedule.user_id)
            .fetch_one(&self.pool)
            .await?;

            let mut anchor_start = if let Some(end_date) = max_end_date {
                end_date
                    .checked_add_days(Days::new(1))
                    .ok_or_else(|| AppError::BadRequest("Invalid end date when generating automatic periods".to_string()))?
            } else {
                compute_initial_anchor_start(
                    today,
                    schedule.start_day,
                    schedule.duration_value,
                    &schedule.duration_unit,
                    schedule.saturday_adjustment,
                    schedule.sunday_adjustment,
                )
                .ok_or_else(|| AppError::BadRequest("Unable to compute automatic period start from schedule".to_string()))?
            };

            for _ in 0..missing {
                let start_date = apply_weekend_adjustment(anchor_start, schedule.saturday_adjustment, schedule.sunday_adjustment)
                    .ok_or_else(|| AppError::BadRequest("Date overflow while applying weekend adjustment".to_string()))?;
                let anchor_end_exclusive = add_duration(anchor_start, schedule.duration_value, &schedule.duration_unit)
                    .ok_or_else(|| AppError::BadRequest("Date overflow while generating automatic period".to_string()))?;
                let raw_end_date = anchor_end_exclusive
                    .checked_sub_days(Days::new(1))
                    .ok_or_else(|| AppError::BadRequest("Invalid period end date during automatic generation".to_string()))?;
                let end_date = apply_weekend_adjustment(raw_end_date, schedule.saturday_adjustment, schedule.sunday_adjustment)
                    .ok_or_else(|| AppError::BadRequest("Date overflow while applying weekend adjustment".to_string()))?;

                let generated_name = render_period_name(&schedule.name_pattern, start_date, end_date);
                let insert_result = sqlx::query_scalar::<_, Uuid>(
                    r#"
                    INSERT INTO budget_period (user_id, name, start_date, end_date, is_auto_generated)
                    VALUES ($1, $2, $3, $4, TRUE)
                    RETURNING id
                    "#,
                )
                .bind(schedule.user_id)
                .bind(&generated_name)
                .bind(start_date)
                .bind(end_date)
                .fetch_one(&self.pool)
                .await;

                match insert_result {
                    Ok(_) => {
                        periods_created += 1;
                    }
                    Err(err) if is_unique_violation(&err) => {
                        let fallback_name = format!("{} ({})", generated_name, start_date.format("%Y-%m-%d"));
                        let fallback_insert = sqlx::query_scalar::<_, Uuid>(
                            r#"
                            INSERT INTO budget_period (user_id, name, start_date, end_date, is_auto_generated)
                            VALUES ($1, $2, $3, $4, TRUE)
                            RETURNING id
                            "#,
                        )
                        .bind(schedule.user_id)
                        .bind(fallback_name)
                        .bind(start_date)
                        .bind(end_date)
                        .fetch_one(&self.pool)
                        .await;

                        match fallback_insert {
                            Ok(_) => periods_created += 1,
                            Err(err) if is_unique_violation(&err) => {}
                            Err(err) => return Err(err.into()),
                        }
                    }
                    Err(err) => return Err(err.into()),
                }

                anchor_start = anchor_end_exclusive;
            }
        }

        Ok(AutoPeriodGenerationResponse {
            users_processed: schedules.len() as i64,
            periods_created,
        })
    }

    // ===== Budget Period with Metrics =====

    pub async fn list_budget_periods_with_metrics(&self, params: &CursorParams, user_id: &Uuid) -> Result<Vec<BudgetPeriodWithMetrics>, AppError> {
        #[derive(sqlx::FromRow)]
        struct PeriodMetricsRow {
            id: Uuid,
            user_id: Uuid,
            name: String,
            start_date: chrono::NaiveDate,
            end_date: chrono::NaiveDate,
            is_auto_generated: bool,
            created_at: chrono::DateTime<chrono::Utc>,
            transaction_count: Option<i64>,
            total_spent: Option<i64>,
            total_budgeted: Option<i64>,
        }

        let rows = if let Some(cursor) = params.cursor {
            sqlx::query_as::<_, PeriodMetricsRow>(
                r#"
                SELECT
                    bp.id,
                    bp.user_id,
                    bp.name,
                    bp.start_date,
                    bp.end_date,
                    bp.is_auto_generated,
                    bp.created_at,
                    COUNT(DISTINCT t.id) as transaction_count,
                    COALESCE(SUM(CASE WHEN c.category_type = 'Outgoing' THEN t.amount ELSE 0 END), 0)::INT8 as total_spent,
                    COALESCE(SUM(bc.budgeted_value), 0) as total_budgeted
                FROM budget_period bp
                LEFT JOIN transaction t ON t.user_id = bp.user_id
                    AND t.occurred_at >= bp.start_date
                    AND t.occurred_at <= bp.end_date
                LEFT JOIN category c ON t.category_id = c.id
                LEFT JOIN budget_category bc ON bc.user_id = bp.user_id
                WHERE bp.user_id = $1
                    AND (bp.start_date, bp.id) > (
                        SELECT start_date, id FROM budget_period WHERE id = $2
                    )
                GROUP BY bp.id, bp.user_id, bp.name, bp.start_date, bp.end_date, bp.is_auto_generated, bp.created_at
                ORDER BY bp.start_date ASC, bp.id ASC
                LIMIT $3
                "#,
            )
            .bind(user_id)
            .bind(cursor)
            .bind(params.fetch_limit())
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, PeriodMetricsRow>(
                r#"
                SELECT
                    bp.id,
                    bp.user_id,
                    bp.name,
                    bp.start_date,
                    bp.end_date,
                    bp.is_auto_generated,
                    bp.created_at,
                    COUNT(DISTINCT t.id) as transaction_count,
                    COALESCE(SUM(CASE WHEN c.category_type = 'Outgoing' THEN t.amount ELSE 0 END), 0)::INT8 as total_spent,
                    COALESCE(SUM(bc.budgeted_value), 0) as total_budgeted
                FROM budget_period bp
                LEFT JOIN transaction t ON t.user_id = bp.user_id
                    AND t.occurred_at >= bp.start_date
                    AND t.occurred_at <= bp.end_date
                LEFT JOIN category c ON t.category_id = c.id
                LEFT JOIN budget_category bc ON bc.user_id = bp.user_id
                WHERE bp.user_id = $1
                GROUP BY bp.id, bp.user_id, bp.name, bp.start_date, bp.end_date, bp.is_auto_generated, bp.created_at
                ORDER BY bp.start_date ASC, bp.id ASC
                LIMIT $2
                "#,
            )
            .bind(user_id)
            .bind(params.fetch_limit())
            .fetch_all(&self.pool)
            .await?
        };

        let periods_with_metrics = rows
            .into_iter()
            .map(|row| {
                let total_spent = row.total_spent.unwrap_or(0);
                let total_budgeted = row.total_budgeted.unwrap_or(0);
                let budget_used_percentage = if total_budgeted > 0 {
                    (total_spent as f64 / total_budgeted as f64) * 100.0
                } else {
                    0.0
                };

                BudgetPeriodWithMetrics {
                    period: BudgetPeriod {
                        id: row.id,
                        user_id: row.user_id,
                        name: row.name,
                        start_date: row.start_date,
                        end_date: row.end_date,
                        is_auto_generated: row.is_auto_generated,
                        created_at: row.created_at,
                    },
                    transaction_count: row.transaction_count.unwrap_or(0),
                    budget_used_percentage,
                }
            })
            .collect();

        Ok(periods_with_metrics)
    }

    pub async fn get_current_budget_period_with_metrics(&self, user_id: &Uuid) -> Result<BudgetPeriodWithMetrics, AppError> {
        #[derive(sqlx::FromRow)]
        struct PeriodMetricsRow {
            id: Uuid,
            user_id: Uuid,
            name: String,
            start_date: chrono::NaiveDate,
            end_date: chrono::NaiveDate,
            is_auto_generated: bool,
            created_at: chrono::DateTime<chrono::Utc>,
            transaction_count: Option<i64>,
            total_spent: Option<i64>,
            total_budgeted: Option<i64>,
        }

        let row = sqlx::query_as::<_, PeriodMetricsRow>(
            r#"
            SELECT
                bp.id,
                bp.user_id,
                bp.name,
                bp.start_date,
                bp.end_date,
                bp.is_auto_generated,
                bp.created_at,
                COUNT(DISTINCT t.id) as transaction_count,
                COALESCE(SUM(CASE WHEN c.category_type = 'Outgoing' THEN t.amount ELSE 0 END), 0)::INT8 as total_spent,
                COALESCE(SUM(bc.budgeted_value), 0) as total_budgeted
            FROM budget_period bp
            LEFT JOIN transaction t ON t.user_id = bp.user_id
                AND t.occurred_at >= bp.start_date
                AND t.occurred_at <= bp.end_date
            LEFT JOIN category c ON t.category_id = c.id
            LEFT JOIN budget_category bc ON bc.user_id = bp.user_id
            WHERE bp.user_id = $1
                AND bp.start_date <= now()
                AND bp.end_date >= now()
            GROUP BY bp.id, bp.user_id, bp.name, bp.start_date, bp.end_date, bp.is_auto_generated, bp.created_at
            "#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        let total_spent = row.total_spent.unwrap_or(0);
        let total_budgeted = row.total_budgeted.unwrap_or(0);
        let budget_used_percentage = if total_budgeted > 0 {
            (total_spent as f64 / total_budgeted as f64) * 100.0
        } else {
            0.0
        };

        Ok(BudgetPeriodWithMetrics {
            period: BudgetPeriod {
                id: row.id,
                user_id: row.user_id,
                name: row.name,
                start_date: row.start_date,
                end_date: row.end_date,
                is_auto_generated: row.is_auto_generated,
                created_at: row.created_at,
            },
            transaction_count: row.transaction_count.unwrap_or(0),
            budget_used_percentage,
        })
    }

    // ===== Gap Detection =====

    pub async fn get_period_gaps(&self, user_id: &Uuid) -> Result<GapsResponse, AppError> {
        #[derive(sqlx::FromRow)]
        struct UnassignedTransactionRow {
            id: Uuid,
            occurred_at: chrono::NaiveDate,
            description: String,
            amount: i64,
        }

        let transactions = sqlx::query_as::<_, UnassignedTransactionRow>(
            r#"
            SELECT t.id, t.occurred_at, t.description, t.amount
            FROM transaction t
            WHERE t.user_id = $1
                AND NOT EXISTS (
                    SELECT 1
                    FROM budget_period bp
                    WHERE bp.user_id = t.user_id
                        AND t.occurred_at >= bp.start_date
                        AND t.occurred_at <= bp.end_date
                )
            ORDER BY t.occurred_at DESC
            LIMIT 100
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        let unassigned_count = transactions.len() as i64;
        let transactions = transactions
            .into_iter()
            .map(|row| UnassignedTransaction {
                id: row.id,
                occurred_at: row.occurred_at,
                description: row.description,
                amount: row.amount,
            })
            .collect();

        Ok(GapsResponse {
            unassigned_count,
            transactions,
        })
    }
}

fn compute_initial_anchor_start(
    today: NaiveDate,
    start_day: i32,
    duration_value: i32,
    duration_unit: &DurationUnit,
    saturday_adjustment: WeekendAdjustment,
    sunday_adjustment: WeekendAdjustment,
) -> Option<NaiveDate> {
    let mut candidate = base_month_start_date(today.year(), today.month(), start_day)
        .and_then(|date| apply_weekend_adjustment(date, saturday_adjustment, sunday_adjustment))?;

    let mut iterations = 0_usize;
    while candidate > today {
        candidate = subtract_duration(candidate, duration_value, duration_unit)?;
        iterations += 1;
        if iterations > 1200 {
            return None;
        }
    }

    Some(candidate)
}

fn add_duration(date: NaiveDate, duration_value: i32, duration_unit: &DurationUnit) -> Option<NaiveDate> {
    match duration_unit {
        DurationUnit::Days => date.checked_add_days(Days::new(duration_value as u64)),
        DurationUnit::Weeks => date.checked_add_days(Days::new((duration_value * 7) as u64)),
        DurationUnit::Months => date.checked_add_months(Months::new(duration_value as u32)),
    }
}

fn subtract_duration(date: NaiveDate, duration_value: i32, duration_unit: &DurationUnit) -> Option<NaiveDate> {
    match duration_unit {
        DurationUnit::Days => date.checked_sub_days(Days::new(duration_value as u64)),
        DurationUnit::Weeks => date.checked_sub_days(Days::new((duration_value * 7) as u64)),
        DurationUnit::Months => date.checked_sub_months(Months::new(duration_value as u32)),
    }
}

fn base_month_start_date(year: i32, month: u32, start_day: i32) -> Option<NaiveDate> {
    let day = start_day.clamp(1, 31) as u32;
    for candidate_day in (1..=day).rev() {
        if let Some(date) = NaiveDate::from_ymd_opt(year, month, candidate_day) {
            return Some(date);
        }
    }
    None
}

fn apply_weekend_adjustment(date: NaiveDate, saturday_adjustment: WeekendAdjustment, sunday_adjustment: WeekendAdjustment) -> Option<NaiveDate> {
    match date.weekday() {
        Weekday::Sat => apply_day_adjustment(date, saturday_adjustment, true),
        Weekday::Sun => apply_day_adjustment(date, sunday_adjustment, false),
        _ => Some(date),
    }
}

fn apply_day_adjustment(date: NaiveDate, adjustment: WeekendAdjustment, is_saturday: bool) -> Option<NaiveDate> {
    match adjustment {
        WeekendAdjustment::Keep => Some(date),
        WeekendAdjustment::Friday => date.checked_sub_days(Days::new(if is_saturday { 1 } else { 2 })),
        WeekendAdjustment::Monday => date.checked_add_days(Days::new(if is_saturday { 2 } else { 1 })),
    }
}

fn render_period_name(pattern: &str, start_date: NaiveDate, end_date: NaiveDate) -> String {
    let rendered = pattern
        .replace("{start_date}", &start_date.format("%Y-%m-%d").to_string())
        .replace("{end_date}", &end_date.format("%Y-%m-%d").to_string())
        .replace("{year}", &start_date.format("%Y").to_string())
        .replace("{month}", &start_date.format("%m").to_string());

    if rendered == pattern {
        format!("{} {}", rendered, start_date.format("%Y-%m-%d"))
    } else {
        rendered
    }
}

#[cfg(test)]
mod tests {
    use super::{apply_weekend_adjustment, render_period_name};
    use crate::models::budget_period::WeekendAdjustment;
    use chrono::NaiveDate;

    #[test]
    fn weekend_adjustment_friday_and_monday() {
        let saturday = NaiveDate::from_ymd_opt(2026, 1, 31).expect("valid date");
        let adjusted = apply_weekend_adjustment(saturday, WeekendAdjustment::Friday, WeekendAdjustment::Keep).expect("adjusted");
        assert_eq!(adjusted, NaiveDate::from_ymd_opt(2026, 1, 30).expect("valid date"));

        let sunday = NaiveDate::from_ymd_opt(2026, 2, 1).expect("valid date");
        let adjusted = apply_weekend_adjustment(sunday, WeekendAdjustment::Keep, WeekendAdjustment::Monday).expect("adjusted");
        assert_eq!(adjusted, NaiveDate::from_ymd_opt(2026, 2, 2).expect("valid date"));
    }

    #[test]
    fn render_name_with_and_without_placeholders() {
        let start = NaiveDate::from_ymd_opt(2026, 2, 1).expect("valid date");
        let end = NaiveDate::from_ymd_opt(2026, 2, 28).expect("valid date");

        let templated = render_period_name("Period {month}/{year}", start, end);
        assert_eq!(templated, "Period 02/2026");

        let plain = render_period_name("Period", start, end);
        assert_eq!(plain, "Period 2026-02-01");
    }
}
