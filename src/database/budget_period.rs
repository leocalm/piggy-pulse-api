use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::models::budget_period::{BudgetPeriod, BudgetPeriodRequest};
use crate::models::pagination::CursorParams;
use uuid::Uuid;

#[async_trait::async_trait]
pub trait BudgetPeriodRepository {
    async fn create_budget_period(&self, request: &BudgetPeriodRequest) -> Result<Uuid, AppError>;
    async fn list_budget_periods(&self, params: &CursorParams) -> Result<Vec<BudgetPeriod>, AppError>;
    async fn get_current_budget_period(&self) -> Result<BudgetPeriod, AppError>;
    async fn get_budget_period(&self, budget_period_id: &Uuid) -> Result<BudgetPeriod, AppError>;
    async fn update_budget_period(&self, id: &Uuid, request: &BudgetPeriodRequest) -> Result<BudgetPeriod, AppError>;
    async fn delete_budget_period(&self, id: &Uuid) -> Result<(), AppError>;
}

#[async_trait::async_trait]
impl BudgetPeriodRepository for PostgresRepository {
    async fn create_budget_period(&self, request: &BudgetPeriodRequest) -> Result<Uuid, AppError> {
        #[derive(sqlx::FromRow)]
        struct IdRow {
            id: Uuid,
        }

        let row = sqlx::query_as::<_, IdRow>(
            r#"
            INSERT INTO budget_period (name, start_date, end_date)
            VALUES ($1, $2, $3)
            RETURNING id
            "#,
        )
        .bind(&request.name)
        .bind(request.start_date)
        .bind(request.end_date)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.id)
    }

    async fn list_budget_periods(&self, params: &CursorParams) -> Result<Vec<BudgetPeriod>, AppError> {
        let budget_periods = if let Some(cursor) = params.cursor {
            sqlx::query_as::<_, BudgetPeriod>(
                r#"
                SELECT id, name, start_date, end_date, created_at
                FROM budget_period
                WHERE (start_date, id) > (
                    SELECT start_date, id FROM budget_period WHERE id = $1
                )
                ORDER BY start_date ASC, id ASC
                LIMIT $2
                "#,
            )
            .bind(cursor)
            .bind(params.fetch_limit())
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, BudgetPeriod>(
                r#"
                SELECT id, name, start_date, end_date, created_at
                FROM budget_period
                ORDER BY start_date ASC, id ASC
                LIMIT $1
                "#,
            )
            .bind(params.fetch_limit())
            .fetch_all(&self.pool)
            .await?
        };

        Ok(budget_periods)
    }

    async fn get_current_budget_period(&self) -> Result<BudgetPeriod, AppError> {
        let budget_period = sqlx::query_as::<_, BudgetPeriod>(
            r#"
            SELECT id, name, start_date, end_date, created_at
            FROM budget_period
            WHERE start_date <= now()
                AND end_date >= now()
            "#,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(budget_period)
    }

    async fn get_budget_period(&self, budget_period_id: &Uuid) -> Result<BudgetPeriod, AppError> {
        let budget_period = sqlx::query_as::<_, BudgetPeriod>(
            r#"
            SELECT id, name, start_date, end_date, created_at
            FROM budget_period
            WHERE id = $1
            "#,
        )
        .bind(budget_period_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(budget_period)
    }

    async fn update_budget_period(&self, id: &Uuid, request: &BudgetPeriodRequest) -> Result<BudgetPeriod, AppError> {
        let budget_period = sqlx::query_as::<_, BudgetPeriod>(
            r#"
            UPDATE budget_period
            SET name = $1, start_date = $2, end_date = $3
            WHERE id = $4
            RETURNING id, name, start_date, end_date, created_at
            "#,
        )
        .bind(&request.name)
        .bind(request.start_date)
        .bind(request.end_date)
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        Ok(budget_period)
    }

    async fn delete_budget_period(&self, id: &Uuid) -> Result<(), AppError> {
        sqlx::query("DELETE FROM budget_period WHERE id = $1").bind(id).execute(&self.pool).await?;

        Ok(())
    }
}
