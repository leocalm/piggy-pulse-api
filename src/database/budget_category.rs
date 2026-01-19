use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::models::budget_category::{BudgetCategory, BudgetCategoryRequest};
use crate::models::category::Category;
use tokio_postgres::Row;
use uuid::Uuid;

#[async_trait::async_trait]
pub trait BudgetCategoryRepository {
    async fn create_budget_category(&self, request: &BudgetCategoryRequest) -> Result<BudgetCategory, AppError>;
    async fn get_budget_category_by_id(&self, id: &Uuid) -> Result<Option<BudgetCategory>, AppError>;
    async fn list_budget_categories(&self) -> Result<Vec<BudgetCategory>, AppError>;
    async fn delete_budget_category(&self, id: &Uuid) -> Result<(), AppError>;
    async fn update_budget_category_value(&self, id: &Uuid, new_budget_value: &i32) -> Result<(), AppError>;
}

#[async_trait::async_trait]
impl<'a> BudgetCategoryRepository for PostgresRepository<'a> {
    async fn create_budget_category(&self, request: &BudgetCategoryRequest) -> Result<BudgetCategory, AppError> {
        let rows = self
            .client
            .query(
                r#"
            INSERT INTO budget_category (category_id, budgeted_value)
            VALUES ($1, $2)
            RETURNING id
            "#,
                &[&request.category_id, &(request.budgeted_value as i32)],
            )
            .await?;

        if let Some(row) = rows.first() {
            match self.get_budget_category_by_id(&row.get("id")).await? {
                None => Err(AppError::Db("Error gettignthe created created budget_category".to_string())),
                Some(new_budget_category) => Ok(new_budget_category),
            }
        } else {
            Err(AppError::Db("Error mapping created budget_category".to_string()))
        }
    }

    async fn get_budget_category_by_id(&self, id: &Uuid) -> Result<Option<BudgetCategory>, AppError> {
        let rows = self
            .client
            .query(
                r#"
            SELECT
                bc.id,
                bc.category_id,
                bc.budgeted_value,
                bc.created_at,
                c.id as category_id,
                c.name as category_name,
                COALESCE(c.color, '') as category_color,
                COALESCE(c.icon, '') as category_icon,
                c.parent_id as category_parent_id,
                c.category_type::text as category_category_type,
                c.created_at as category_created_at
            FROM budget_category bc
            JOIN category c
                ON c.id = bc.category_id
            WHERE bc.id = $1
            "#,
                &[id],
            )
            .await?;

        if let Some(row) = rows.first() {
            Ok(Some(map_row_to_budget_category(row)))
        } else {
            Ok(None)
        }
    }

    async fn list_budget_categories(&self) -> Result<Vec<BudgetCategory>, AppError> {
        Ok(self
            .client
            .query(
                r#"
            SELECT
                bc.id,
                bc.category_id,
                bc.budgeted_value,
                bc.created_at,
                c.id as category_id,
                c.name as category_name,
                COALESCE(c.color, '') as category_color,
                COALESCE(c.icon, '') as category_icon,
                c.parent_id as category_parent_id,
                c.category_type::text as category_category_type,
                c.created_at as category_created_at
            FROM budget_category bc
            JOIN category c
                ON c.id = bc.category_id
            ORDER BY bc.created_at DESC
            "#,
                &[],
            )
            .await?
            .into_iter()
            .map(|r| map_row_to_budget_category(&r))
            .collect())
    }

    async fn delete_budget_category(&self, id: &Uuid) -> Result<(), AppError> {
        self.client.execute(r#"DELETE FROM budget_category WHERE id = $1"#, &[id]).await?;
        Ok(())
    }

    async fn update_budget_category_value(&self, id: &Uuid, new_budget_value: &i32) -> Result<(), AppError> {
        self.client
            .execute(r#"UPDATE budget_category SET budgeted_value = $2 WHERE id = $1"#, &[id, &new_budget_value])
            .await?;
        Ok(())
    }
}

fn map_row_to_budget_category(row: &Row) -> BudgetCategory {
    BudgetCategory {
        id: row.get("id"),
        category_id: row.get("category_id"),
        budgeted_value: row.get::<_, i32>("budgeted_value"),
        created_at: row.get("created_at"),
        category: Category {
            id: row.get("category_id"),
            name: row.get("category_name"),
            color: row.get("category_color"),
            icon: row.get("category_icon"),
            parent_id: row.get("category_parent_id"),
            category_type: crate::database::category::category_type_from_db(row.get::<_, &str>("category_category_type")),
            created_at: row.get("category_created_at"),
        },
    }
}
