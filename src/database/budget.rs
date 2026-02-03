use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::models::budget::{Budget, BudgetRequest};
use crate::models::pagination::CursorParams;
use uuid::Uuid;

#[async_trait::async_trait]
pub trait BudgetRepository {
    async fn create_budget(&self, request: &BudgetRequest, user_id: &Uuid) -> Result<Budget, AppError>;
    async fn get_budget_by_id(&self, id: &Uuid, user_id: &Uuid) -> Result<Option<Budget>, AppError>;
    async fn list_budgets(&self, params: &CursorParams, user_id: &Uuid) -> Result<Vec<Budget>, AppError>;
    async fn delete_budget(&self, id: &Uuid, user_id: &Uuid) -> Result<(), AppError>;
    async fn update_budget(&self, id: &Uuid, budget: &BudgetRequest, user_id: &Uuid) -> Result<Budget, AppError>;
}

#[async_trait::async_trait]
impl BudgetRepository for PostgresRepository {
    async fn create_budget(&self, request: &BudgetRequest, user_id: &Uuid) -> Result<Budget, AppError> {
        let budget = sqlx::query_as::<_, Budget>(
            r#"
            INSERT INTO budget (user_id, name, start_day)
            VALUES ($1, $2, $3)
            RETURNING id, user_id, name, start_day, created_at
            "#,
        )
        .bind(user_id)
        .bind(&request.name)
        .bind(request.start_day)
        .fetch_one(&self.pool)
        .await?;

        Ok(budget)
    }

    async fn get_budget_by_id(&self, id: &Uuid, user_id: &Uuid) -> Result<Option<Budget>, AppError> {
        let budget = sqlx::query_as::<_, Budget>(
            r#"
            SELECT id, user_id, name, start_day, created_at
            FROM budget
            WHERE id = $1 AND user_id = $2
            "#,
        )
        .bind(id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(budget)
    }

    async fn list_budgets(&self, params: &CursorParams, user_id: &Uuid) -> Result<Vec<Budget>, AppError> {
        let budgets = if let Some(cursor) = params.cursor {
            sqlx::query_as::<_, Budget>(
                r#"
                SELECT id, user_id, name, start_day, created_at
                FROM budget
                WHERE user_id = $1
                    AND (created_at, id) < (SELECT created_at, id FROM budget WHERE id = $2)
                ORDER BY created_at DESC, id DESC
                LIMIT $3
                "#,
            )
            .bind(user_id)
            .bind(cursor)
            .bind(params.fetch_limit())
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, Budget>(
                r#"
                SELECT id, user_id, name, start_day, created_at
                FROM budget
                WHERE user_id = $1
                ORDER BY created_at DESC, id DESC
                LIMIT $2
                "#,
            )
            .bind(user_id)
            .bind(params.fetch_limit())
            .fetch_all(&self.pool)
            .await?
        };

        Ok(budgets)
    }

    async fn delete_budget(&self, id: &Uuid, user_id: &Uuid) -> Result<(), AppError> {
        sqlx::query("DELETE FROM budget WHERE id = $1 AND user_id = $2")
            .bind(id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn update_budget(&self, id: &Uuid, budget: &BudgetRequest, user_id: &Uuid) -> Result<Budget, AppError> {
        let updated_budget = sqlx::query_as::<_, Budget>(
            r#"
            UPDATE budget
            SET name = $1, start_day = $2
            WHERE id = $3 AND user_id = $4
            RETURNING id, user_id, name, start_day, created_at
            "#,
        )
        .bind(&budget.name)
        .bind(budget.start_day)
        .bind(id)
        .bind(user_id)
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

        let result = repo.create_budget(&request, &Uuid::new_v4()).await;
        assert!(result.is_ok());
        let budget = result.unwrap();
        assert_eq!(budget.name, "Monthly Budget");
        assert_eq!(budget.start_day, 1);
    }

    #[tokio::test]
    async fn test_mock_get_budget_by_id() {
        let repo = MockRepository {};
        let id = Uuid::new_v4();

        let result = repo.get_budget_by_id(&id, &Uuid::new_v4()).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());
    }

    #[tokio::test]
    async fn test_mock_list_budgets() {
        let repo = MockRepository {};
        let params = crate::models::pagination::CursorParams { cursor: None, limit: None };
        let result = repo.list_budgets(&params, &Uuid::new_v4()).await;
        assert!(result.is_ok());
        let budgets = result.unwrap();
        assert_eq!(budgets.len(), 1);
    }

    #[tokio::test]
    async fn test_mock_delete_budget() {
        let repo = MockRepository {};
        let id = Uuid::new_v4();
        let result = repo.delete_budget(&id, &Uuid::new_v4()).await;
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

        let result = repo.update_budget(&id, &request, &Uuid::new_v4()).await;
        assert!(result.is_ok());
        let budget = result.unwrap();
        assert_eq!(budget.id, id);
        assert_eq!(budget.name, "Updated Budget");
        assert_eq!(budget.start_day, 15);
    }
}
