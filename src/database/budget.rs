use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::models::budget::{Budget, BudgetRequest};
use crate::models::pagination::PaginationParams;
use tokio_postgres::Row;
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
                &[&request.name, &{ request.start_day }],
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

    async fn list_budgets(&self, pagination: Option<&PaginationParams>) -> Result<(Vec<Budget>, i64), AppError> {
        // Get total count
        let count_row = self.client.query_one("SELECT COUNT(*) as total FROM budget", &[]).await?;
        let total: i64 = count_row.get("total");

        // Build query with optional pagination
        let mut query = String::from(
            r#"
            SELECT id, name, start_day, created_at
            FROM budget
            ORDER BY created_at DESC
            "#,
        );

        // Add pagination if requested
        let rows = if let Some(params) = pagination {
            if let (Some(limit), Some(offset)) = (params.effective_limit(), params.offset()) {
                query.push_str(&format!(" LIMIT {} OFFSET {}", limit, offset));
                self.client.query(&query, &[]).await?
            } else {
                self.client.query(&query, &[]).await?
            }
        } else {
            self.client.query(&query, &[]).await?
        };

        Ok((rows.into_iter().map(|r| map_row_to_budget(&r)).collect(), total))
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
                &[&budget.name, &{ budget.start_day }, &id],
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
