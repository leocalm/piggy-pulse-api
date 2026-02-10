use crate::database::postgres_repository::{PostgresRepository, is_unique_violation};
use crate::error::app_error::AppError;
use crate::models::budget_period::{
    BudgetPeriod, BudgetPeriodRequest, BudgetPeriodWithMetrics, GapsResponse, PeriodSchedule, PeriodScheduleRequest, UnassignedTransaction,
};
use crate::models::pagination::CursorParams;
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
