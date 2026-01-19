use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::models::budget::{Budget, BudgetRequest};
use tokio_postgres::Row;
use uuid::Uuid;

#[async_trait::async_trait]
pub trait BudgetRepository {
    async fn create_budget(&self, request: &BudgetRequest) -> Result<Budget, AppError>;
    async fn get_budget_by_id(&self, id: &Uuid) -> Result<Option<Budget>, AppError>;
    async fn list_budgets(&self) -> Result<Vec<Budget>, AppError>;
    async fn delete_budget(&self, id: &Uuid) -> Result<(), AppError>;
    async fn update_budget(&self, id: &Uuid, budget: &BudgetRequest) -> Result<Budget, AppError>;
}

#[async_trait::async_trait]
impl<'a> BudgetRepository for PostgresRepository<'a> {
    async fn create_budget(&self, request: &BudgetRequest) -> Result<Budget, AppError> {
        let rows = self
            .client
            .query(
                r#"
            INSERT INTO budget (name, start_day)
            VALUES ($1, $2)
            RETURNING id, name, start_day, created_at
            "#,
                &[&request.name, &(request.start_day as i32)],
            )
            .await?;

        if let Some(row) = rows.first() {
            Ok(map_row_to_budget(row))
        } else {
            Err(AppError::Db("Error mapping created budget".to_string()))
        }
    }

    async fn get_budget_by_id(&self, id: &Uuid) -> Result<Option<Budget>, AppError> {
        let rows = self
            .client
            .query(
                r#"
            SELECT id, name, start_day, created_at
            FROM budget
            WHERE id = $1
            "#,
                &[id],
            )
            .await?;

        if let Some(row) = rows.first() {
            Ok(Some(map_row_to_budget(row)))
        } else {
            Ok(None)
        }
    }

    async fn list_budgets(&self) -> Result<Vec<Budget>, AppError> {
        let rows = self
            .client
            .query(
                r#"
            SELECT id, name, start_day, created_at
            FROM budget
            ORDER BY created_at DESC
            "#,
                &[],
            )
            .await?;

        Ok(rows.into_iter().map(|r| map_row_to_budget(&r)).collect())
    }

    async fn delete_budget(&self, id: &Uuid) -> Result<(), AppError> {
        self.client
            .execute(
                r#"
            DELETE FROM budget
            WHERE id = $1
            "#,
                &[id],
            )
            .await?;
        Ok(())
    }

    async fn update_budget(&self, id: &Uuid, budget: &BudgetRequest) -> Result<Budget, AppError> {
        let rows = self
            .client
            .query(
                r#"
            UPDATE budget
            SET name = $1, start_day = $2
            WHERE id = $3
            RETURNING id, name, start_day, created_at
            "#,
                &[&budget.name, &(budget.start_day as i32), &id],
            )
            .await?;

        if let Some(row) = rows.first() {
            Ok(map_row_to_budget(row))
        } else {
            Err(AppError::Db("Error mapping created budget".to_string()))
        }
    }
}

fn map_row_to_budget(row: &Row) -> Budget {
    Budget {
        id: row.get("id"),
        name: row.get("name"),
        start_day: row.get::<_, i32>("start_day"),
        created_at: row.get("created_at"),
    }
}
