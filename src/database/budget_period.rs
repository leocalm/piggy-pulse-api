use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::models::budget_period::{BudgetPeriod, BudgetPeriodRequest};
use crate::models::pagination::CursorParams;
use uuid::Uuid;

impl PostgresRepository {
    pub async fn create_budget_period(&self, request: &BudgetPeriodRequest, user_id: &Uuid) -> Result<Uuid, AppError> {
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
        .await?;

        Ok(row.id)
    }

    pub async fn list_budget_periods(&self, params: &CursorParams, user_id: &Uuid) -> Result<Vec<BudgetPeriod>, AppError> {
        let budget_periods = if let Some(cursor) = params.cursor {
            sqlx::query_as::<_, BudgetPeriod>(
                r#"
                SELECT id, user_id, name, start_date, end_date, created_at
                FROM budget_period
                WHERE user_id = $1
                    AND (start_date, id) > (
                        SELECT start_date, id FROM budget_period WHERE id = $2
                    )
                ORDER BY start_date ASC, id ASC
                LIMIT $3
                "#,
            )
            .bind(user_id)
            .bind(cursor)
            .bind(params.fetch_limit())
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, BudgetPeriod>(
                r#"
                SELECT id, user_id, name, start_date, end_date, created_at
                FROM budget_period
                WHERE user_id = $1
                ORDER BY start_date ASC, id ASC
                LIMIT $2
                "#,
            )
            .bind(user_id)
            .bind(params.fetch_limit())
            .fetch_all(&self.pool)
            .await?
        };

        Ok(budget_periods)
    }

    pub async fn get_current_budget_period(&self, user_id: &Uuid) -> Result<BudgetPeriod, AppError> {
        let budget_period = sqlx::query_as::<_, BudgetPeriod>(
            r#"
            SELECT id, user_id, name, start_date, end_date, created_at
            FROM budget_period
            WHERE user_id = $1
                AND start_date <= now()
                AND end_date >= now()
            "#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(budget_period)
    }

    pub async fn get_budget_period(&self, budget_period_id: &Uuid, user_id: &Uuid) -> Result<BudgetPeriod, AppError> {
        let budget_period = sqlx::query_as::<_, BudgetPeriod>(
            r#"
            SELECT id, user_id, name, start_date, end_date, created_at
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
        let budget_period = sqlx::query_as::<_, BudgetPeriod>(
            r#"
            UPDATE budget_period
            SET name = $1, start_date = $2, end_date = $3
            WHERE id = $4 AND user_id = $5
            RETURNING id, user_id, name, start_date, end_date, created_at
            "#,
        )
        .bind(&request.name)
        .bind(request.start_date)
        .bind(request.end_date)
        .bind(id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

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
}
