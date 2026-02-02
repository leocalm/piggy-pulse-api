use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::models::category::{Category, CategoryRequest, CategoryType};
use crate::models::pagination::PaginationParams;
use chrono::{DateTime, Utc};
use uuid::Uuid;

// Intermediate struct for sqlx query results with category_type as text
#[derive(Debug, sqlx::FromRow)]
struct CategoryRow {
    id: Uuid,
    name: String,
    color: String,
    icon: String,
    parent_id: Option<Uuid>,
    category_type: String,
    created_at: DateTime<Utc>,
}

impl From<CategoryRow> for Category {
    fn from(row: CategoryRow) -> Self {
        Category {
            id: row.id,
            name: row.name,
            color: row.color,
            icon: row.icon,
            parent_id: row.parent_id,
            category_type: category_type_from_db(&row.category_type),
            created_at: row.created_at,
        }
    }
}

#[async_trait::async_trait]
pub trait CategoryRepository {
    async fn create_category(&self, request: &CategoryRequest) -> Result<Category, AppError>;
    async fn get_category_by_id(&self, id: &Uuid) -> Result<Option<Category>, AppError>;
    async fn list_categories(&self, pagination: Option<&PaginationParams>) -> Result<(Vec<Category>, i64), AppError>;
    async fn delete_category(&self, id: &Uuid) -> Result<(), AppError>;
    async fn update_category(&self, id: &Uuid, request: &CategoryRequest) -> Result<Category, AppError>;
    async fn list_categories_not_in_budget(&self, pagination: Option<&PaginationParams>) -> Result<(Vec<Category>, i64), AppError>;
}

#[async_trait::async_trait]
impl CategoryRepository for PostgresRepository {
    async fn create_category(&self, request: &CategoryRequest) -> Result<Category, AppError> {
        let row = sqlx::query_as::<_, CategoryRow>(
            r#"
            INSERT INTO category (name, color, icon, parent_id, category_type)
            VALUES ($1, $2, $3, $4, $5::text::category_type)
            RETURNING
                id,
                name,
                COALESCE(color, '') as color,
                COALESCE(icon, '') as icon,
                parent_id,
                category_type::text as category_type,
                created_at
            "#,
        )
        .bind(&request.name)
        .bind(&request.color)
        .bind(&request.icon)
        .bind(request.parent_id)
        .bind(request.category_type_to_db())
        .fetch_one(&self.pool)
        .await?;

        Ok(Category::from(row))
    }

    async fn get_category_by_id(&self, id: &Uuid) -> Result<Option<Category>, AppError> {
        let row = sqlx::query_as::<_, CategoryRow>(
            r#"
            SELECT
                id,
                name,
                COALESCE(color, '') as color,
                COALESCE(icon, '') as icon,
                parent_id,
                category_type::text as category_type,
                created_at
            FROM category
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Category::from))
    }

    async fn list_categories(&self, pagination: Option<&PaginationParams>) -> Result<(Vec<Category>, i64), AppError> {
        // Get total count
        #[derive(sqlx::FromRow)]
        struct CountRow {
            total: i64,
        }

        let count_row = sqlx::query_as::<_, CountRow>("SELECT COUNT(*) as total FROM category")
            .fetch_one(&self.pool)
            .await?;
        let total = count_row.total;

        // Build query with optional pagination
        let mut query = String::from(
            r#"
            SELECT
                id,
                name,
                COALESCE(color, '') as color,
                COALESCE(icon, '') as icon,
                parent_id,
                category_type::text as category_type,
                created_at
            FROM category
            ORDER BY created_at DESC
            "#,
        );

        // Add pagination if requested
        if let Some(params) = pagination
            && let (Some(limit), Some(offset)) = (params.effective_limit(), params.offset())
        {
            query.push_str(&format!(" LIMIT {} OFFSET {}", limit, offset));
        }

        let rows = sqlx::query_as::<_, CategoryRow>(&query).fetch_all(&self.pool).await?;

        let categories: Vec<Category> = rows.into_iter().map(Category::from).collect();

        Ok((categories, total))
    }

    async fn delete_category(&self, id: &Uuid) -> Result<(), AppError> {
        sqlx::query("DELETE FROM category WHERE id = $1").bind(id).execute(&self.pool).await?;

        Ok(())
    }

    async fn update_category(&self, id: &Uuid, request: &CategoryRequest) -> Result<Category, AppError> {
        let row = sqlx::query_as::<_, CategoryRow>(
            r#"
            UPDATE category
            SET name = $1, color = $2, icon = $3, parent_id = $4, category_type = $5::text::category_type
            WHERE id = $6
            RETURNING
                id,
                name,
                COALESCE(color, '') as color,
                COALESCE(icon, '') as icon,
                parent_id,
                category_type::text as category_type,
                created_at
            "#,
        )
        .bind(&request.name)
        .bind(&request.color)
        .bind(&request.icon)
        .bind(request.parent_id)
        .bind(request.category_type_to_db())
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        Ok(Category::from(row))
    }

    async fn list_categories_not_in_budget(&self, pagination: Option<&PaginationParams>) -> Result<(Vec<Category>, i64), AppError> {
        // Get total count
        #[derive(sqlx::FromRow)]
        struct CountRow {
            total: i64,
        }

        let count_row = sqlx::query_as::<_, CountRow>(
            r#"
            SELECT COUNT(*) as total
            FROM category c
            LEFT JOIN budget_category bc
                ON c.id = bc.category_id
            WHERE bc.id is null
                AND c.category_type = 'Outgoing'
            "#,
        )
        .fetch_one(&self.pool)
        .await?;
        let total = count_row.total;

        // Build query with optional pagination
        let mut query = String::from(
            r#"
            SELECT
                c.id,
                c.name,
                COALESCE(c.color, '') as color,
                COALESCE(c.icon, '') as icon,
                c.parent_id,
                c.category_type::text as category_type,
                c.created_at
            FROM category c
            LEFT JOIN budget_category bc
                ON c.id = bc.category_id
            WHERE bc.id is null
                AND c.category_type = 'Outgoing'
            ORDER BY created_at DESC
            "#,
        );

        // Add pagination if requested
        if let Some(params) = pagination
            && let (Some(limit), Some(offset)) = (params.effective_limit(), params.offset())
        {
            query.push_str(&format!(" LIMIT {} OFFSET {}", limit, offset));
        }

        let rows = sqlx::query_as::<_, CategoryRow>(&query).fetch_all(&self.pool).await?;

        let categories: Vec<Category> = rows.into_iter().map(Category::from).collect();

        Ok((categories, total))
    }
}

pub fn category_type_from_db<T: AsRef<str>>(value: T) -> CategoryType {
    match value.as_ref() {
        "Incoming" => CategoryType::Incoming,
        "Outgoing" => CategoryType::Outgoing,
        "Transfer" => CategoryType::Transfer,
        other => panic!("Unknown category type: {}", other),
    }
}

trait CategoryRequestDbExt {
    fn category_type_to_db(&self) -> String;
}

impl CategoryRequestDbExt for CategoryRequest {
    fn category_type_to_db(&self) -> String {
        match self.category_type {
            CategoryType::Incoming => "Incoming".to_string(),
            CategoryType::Outgoing => "Outgoing".to_string(),
            CategoryType::Transfer => "Transfer".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_category_type_from_db_all_types() {
        assert!(matches!(category_type_from_db("Incoming"), CategoryType::Incoming));
        assert!(matches!(category_type_from_db("Outgoing"), CategoryType::Outgoing));
        assert!(matches!(category_type_from_db("Transfer"), CategoryType::Transfer));
    }

    #[test]
    #[should_panic(expected = "Unknown category type")]
    fn test_category_type_from_db_invalid() {
        category_type_from_db("InvalidType");
    }

    #[test]
    fn test_category_type_to_db() {
        let request = CategoryRequest {
            name: "Test".to_string(),
            color: "#000000".to_string(),
            icon: "icon".to_string(),
            parent_id: None,
            category_type: CategoryType::Incoming,
        };
        assert_eq!(request.category_type_to_db(), "Incoming");

        let request_outgoing = CategoryRequest {
            name: "Test".to_string(),
            color: "#000000".to_string(),
            icon: "icon".to_string(),
            parent_id: None,
            category_type: CategoryType::Outgoing,
        };
        assert_eq!(request_outgoing.category_type_to_db(), "Outgoing");

        let request_transfer = CategoryRequest {
            name: "Test".to_string(),
            color: "#000000".to_string(),
            icon: "icon".to_string(),
            parent_id: None,
            category_type: CategoryType::Transfer,
        };
        assert_eq!(request_transfer.category_type_to_db(), "Transfer");
    }
}
