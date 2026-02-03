use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::models::budget::{Budget, BudgetRequest};
use crate::models::pagination::PaginationParams;
use uuid::Uuid;

#[async_trait::async_trait]
pub trait BudgetRepository {
    async fn create_budget(&self, request: &BudgetRequest) -> Result<Budget, AppError>;
    async fn get_budget_by_id(&self, id: &Uuid) -> Result<Option<Budget>, AppError>;
    async fn list_budgets(&self, pagination: Option<&PaginationParams>) -> Result<(Vec<Budget>, i64), AppError>;
    async fn delete_budget(&self, id: &Uuid) -> Result<(), AppError>;
    async fn update_budget(&self, id: &Uuid, budget: &BudgetRequest) -> Result<Budget, AppError>;
}

#[async_trait::async_trait]
impl BudgetRepository for PostgresRepository {
    async fn create_budget(&self, request: &BudgetRequest) -> Result<Budget, AppError> {
        let budget = sqlx::query_as::<_, Budget>(
            r#"
            INSERT INTO budget (name, start_day)
            VALUES ($1, $2)
            RETURNING id, name, start_day, created_at
            "#,
        )
        .bind(&request.name)
        .bind(request.start_day)
        .fetch_one(&self.pool)
        .await?;

        Ok(budget)
    }

    async fn get_budget_by_id(&self, id: &Uuid) -> Result<Option<Budget>, AppError> {
        let budget = sqlx::query_as::<_, Budget>(
            r#"
            SELECT id, name, start_day, created_at
            FROM budget
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(budget)
    }

    async fn list_budgets(&self, pagination: Option<&PaginationParams>) -> Result<(Vec<Budget>, i64), AppError> {
        // Get total count
        #[derive(sqlx::FromRow)]
        struct CountRow {
            total: i64,
        }

        let count_row = sqlx::query_as::<_, CountRow>("SELECT COUNT(*) as total FROM budget")
            .fetch_one(&self.pool)
            .await?;
        let total = count_row.total;

        // Build query with optional pagination
        let base_query = r#"
            SELECT id, name, start_day, created_at
            FROM budget
            ORDER BY created_at DESC
            "#;

        let budgets = if let Some(params) = pagination
            && let (Some(limit), Some(offset)) = (params.effective_limit(), params.offset())
        {
            sqlx::query_as::<_, Budget>(&format!("{} LIMIT $1 OFFSET $2", base_query))
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await?
        } else {
            sqlx::query_as::<_, Budget>(base_query).fetch_all(&self.pool).await?
        };

        Ok((budgets, total))
    }

    async fn delete_budget(&self, id: &Uuid) -> Result<(), AppError> {
        sqlx::query("DELETE FROM budget WHERE id = $1").bind(id).execute(&self.pool).await?;

        Ok(())
    }

    async fn update_budget(&self, id: &Uuid, budget: &BudgetRequest) -> Result<Budget, AppError> {
        let updated_budget = sqlx::query_as::<_, Budget>(
            r#"
            UPDATE budget
            SET name = $1, start_day = $2
            WHERE id = $3
            RETURNING id, name, start_day, created_at
            "#,
        )
        .bind(&budget.name)
        .bind(budget.start_day)
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        Ok(updated_budget)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::MockRepository;

    #[tokio::test]
    async fn test_mock_create_budget() {
        let repo = MockRepository {};
        let request = BudgetRequest {
            name: "Monthly Budget".to_string(),
            start_day: 1,
        };

        let result = repo.create_budget(&request).await;
        assert!(result.is_ok());
        let budget = result.unwrap();
        assert_eq!(budget.name, "Monthly Budget");
        assert_eq!(budget.start_day, 1);
    }

    #[tokio::test]
    async fn test_mock_get_budget_by_id() {
        let repo = MockRepository {};
        let id = Uuid::new_v4();

        let result = repo.get_budget_by_id(&id).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());
    }

    #[tokio::test]
    async fn test_mock_list_budgets() {
        let repo = MockRepository {};
        let result = repo.list_budgets(None).await;
        assert!(result.is_ok());
        let (budgets, total) = result.unwrap();
        assert_eq!(total, 1);
        assert_eq!(budgets.len(), 1);
    }

    #[tokio::test]
    async fn test_mock_delete_budget() {
        let repo = MockRepository {};
        let id = Uuid::new_v4();
        let result = repo.delete_budget(&id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_update_budget() {
        let repo = MockRepository {};
        let id = Uuid::new_v4();
        let request = BudgetRequest {
            name: "Updated Budget".to_string(),
            start_day: 15,
        };

        let result = repo.update_budget(&id, &request).await;
        assert!(result.is_ok());
        let budget = result.unwrap();
        assert_eq!(budget.id, id);
        assert_eq!(budget.name, "Updated Budget");
        assert_eq!(budget.start_day, 15);
    }
}
