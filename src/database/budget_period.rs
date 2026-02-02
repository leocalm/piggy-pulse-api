use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::models::budget_period::{BudgetPeriod, BudgetPeriodRequest};
use crate::models::pagination::PaginationParams;
use uuid::Uuid;

#[async_trait::async_trait]
pub trait BudgetPeriodRepository {
    async fn create_budget_period(&self, request: &BudgetPeriodRequest) -> Result<Uuid, AppError>;
    async fn list_budget_periods(&self, pagination: Option<&PaginationParams>) -> Result<(Vec<BudgetPeriod>, i64), AppError>;
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

    async fn list_budget_periods(&self, pagination: Option<&PaginationParams>) -> Result<(Vec<BudgetPeriod>, i64), AppError> {
        // Get total count
        #[derive(sqlx::FromRow)]
        struct CountRow {
            total: i64,
        }

        let count_row = sqlx::query_as::<_, CountRow>("SELECT COUNT(*) as total FROM budget_period")
            .fetch_one(&self.pool)
            .await?;
        let total = count_row.total;

        // Build query with optional pagination
        let base_query = r#"
            SELECT id, name, start_date, end_date, created_at
            FROM budget_period
            ORDER BY start_date
            "#;

        let budget_periods = if let Some(params) = pagination
            && let (Some(limit), Some(offset)) = (params.effective_limit(), params.offset())
        {
            sqlx::query_as::<_, BudgetPeriod>(&format!("{} LIMIT $1 OFFSET $2", base_query))
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await?
        } else {
            sqlx::query_as::<_, BudgetPeriod>(base_query).fetch_all(&self.pool).await?
        };

        Ok((budget_periods, total))
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
