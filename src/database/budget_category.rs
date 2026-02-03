use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::models::budget_category::{BudgetCategory, BudgetCategoryRequest};
use crate::models::category::Category;
use crate::models::pagination::CursorParams;
use chrono::{DateTime, Utc};
use uuid::Uuid;

// Intermediate struct for sqlx query results with JOINed category data
#[derive(Debug, sqlx::FromRow)]
struct BudgetCategoryRow {
    id: Uuid,
    user_id: Uuid,
    category_id: Uuid,
    budgeted_value: i32,
    created_at: DateTime<Utc>,
    category_name: String,
    category_color: String,
    category_icon: String,
    category_parent_id: Option<Uuid>,
    category_category_type: String,
    category_created_at: DateTime<Utc>,
}

impl From<BudgetCategoryRow> for BudgetCategory {
    fn from(row: BudgetCategoryRow) -> Self {
        BudgetCategory {
            id: row.id,
            user_id: row.user_id,
            category_id: row.category_id,
            budgeted_value: row.budgeted_value,
            created_at: row.created_at,
            category: Category {
                id: row.category_id,
                user_id: Uuid::nil(),
                name: row.category_name,
                color: row.category_color,
                icon: row.category_icon,
                parent_id: row.category_parent_id,
                category_type: crate::database::category::category_type_from_db(&row.category_category_type),
                created_at: row.category_created_at,
            },
        }
    }
}

#[async_trait::async_trait]
pub trait BudgetCategoryRepository {
    async fn create_budget_category(&self, request: &BudgetCategoryRequest, user_id: &Uuid) -> Result<BudgetCategory, AppError>;
    async fn get_budget_category_by_id(&self, id: &Uuid, user_id: &Uuid) -> Result<Option<BudgetCategory>, AppError>;
    async fn list_budget_categories(&self, params: &CursorParams, user_id: &Uuid) -> Result<Vec<BudgetCategory>, AppError>;
    async fn delete_budget_category(&self, id: &Uuid, user_id: &Uuid) -> Result<(), AppError>;
    async fn update_budget_category_value(&self, id: &Uuid, new_budget_value: &i32, user_id: &Uuid) -> Result<(), AppError>;
}

#[async_trait::async_trait]
impl BudgetCategoryRepository for PostgresRepository {
    async fn create_budget_category(&self, request: &BudgetCategoryRequest, user_id: &Uuid) -> Result<BudgetCategory, AppError> {
        #[derive(sqlx::FromRow)]
        struct IdRow {
            id: Uuid,
        }

        let row = sqlx::query_as::<_, IdRow>(
            r#"
            INSERT INTO budget_category (user_id, category_id, budgeted_value)
            VALUES ($1, $2, $3)
            RETURNING id
            "#,
        )
        .bind(user_id)
        .bind(request.category_id)
        .bind(request.budgeted_value)
        .fetch_one(&self.pool)
        .await?;

        match self.get_budget_category_by_id(&row.id, user_id).await? {
            None => Err(AppError::BadRequest("Failed to retrieve newly created budget category".to_string())),
            Some(new_budget_category) => Ok(new_budget_category),
        }
    }

    async fn get_budget_category_by_id(&self, id: &Uuid, user_id: &Uuid) -> Result<Option<BudgetCategory>, AppError> {
        let row = sqlx::query_as::<_, BudgetCategoryRow>(
            r#"
            SELECT
                bc.id,
                bc.user_id,
                bc.category_id,
                bc.budgeted_value,
                bc.created_at,
                c.name as category_name,
                COALESCE(c.color, '') as category_color,
                COALESCE(c.icon, '') as category_icon,
                c.parent_id as category_parent_id,
                c.category_type::text as category_category_type,
                c.created_at as category_created_at
            FROM budget_category bc
            JOIN category c
                ON c.id = bc.category_id
            WHERE bc.id = $1 AND bc.user_id = $2
            "#,
        )
        .bind(id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(BudgetCategory::from))
    }

    async fn list_budget_categories(&self, params: &CursorParams, user_id: &Uuid) -> Result<Vec<BudgetCategory>, AppError> {
        let rows = if let Some(cursor) = params.cursor {
            sqlx::query_as::<_, BudgetCategoryRow>(
                r#"
                SELECT
                    bc.id,
                    bc.user_id,
                    bc.category_id,
                    bc.budgeted_value,
                    bc.created_at,
                    c.name as category_name,
                    COALESCE(c.color, '') as category_color,
                    COALESCE(c.icon, '') as category_icon,
                    c.parent_id as category_parent_id,
                    c.category_type::text as category_category_type,
                    c.created_at as category_created_at
                FROM budget_category bc
                JOIN category c ON c.id = bc.category_id
                WHERE bc.user_id = $1
                    AND (bc.created_at, bc.id) < (
                        SELECT created_at, id FROM budget_category WHERE id = $2
                    )
                ORDER BY bc.created_at DESC, bc.id DESC
                LIMIT $3
                "#,
            )
            .bind(user_id)
            .bind(cursor)
            .bind(params.fetch_limit())
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, BudgetCategoryRow>(
                r#"
                SELECT
                    bc.id,
                    bc.user_id,
                    bc.category_id,
                    bc.budgeted_value,
                    bc.created_at,
                    c.name as category_name,
                    COALESCE(c.color, '') as category_color,
                    COALESCE(c.icon, '') as category_icon,
                    c.parent_id as category_parent_id,
                    c.category_type::text as category_category_type,
                    c.created_at as category_created_at
                FROM budget_category bc
                JOIN category c ON c.id = bc.category_id
                WHERE bc.user_id = $1
                ORDER BY bc.created_at DESC, bc.id DESC
                LIMIT $2
                "#,
            )
            .bind(user_id)
            .bind(params.fetch_limit())
            .fetch_all(&self.pool)
            .await?
        };

        Ok(rows.into_iter().map(BudgetCategory::from).collect())
    }

    async fn delete_budget_category(&self, id: &Uuid, user_id: &Uuid) -> Result<(), AppError> {
        sqlx::query("DELETE FROM budget_category WHERE id = $1 AND user_id = $2")
            .bind(id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn update_budget_category_value(&self, id: &Uuid, new_budget_value: &i32, user_id: &Uuid) -> Result<(), AppError> {
        sqlx::query("UPDATE budget_category SET budgeted_value = $1 WHERE id = $2 AND user_id = $3")
            .bind(new_budget_value)
            .bind(id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}
