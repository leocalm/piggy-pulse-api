use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::models::category::{Category, CategoryRequest, CategoryType};
use crate::models::pagination::CursorParams;
use chrono::{DateTime, Utc};
use uuid::Uuid;

// Intermediate struct for sqlx query results with category_type as text
#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct CategoryRow {
    id: Uuid,
    user_id: Uuid,
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
            user_id: row.user_id,
            name: row.name,
            color: row.color,
            icon: row.icon,
            parent_id: row.parent_id,
            category_type: category_type_from_db(&row.category_type),
            created_at: row.created_at,
        }
    }
}

impl PostgresRepository {
    pub async fn create_category(&self, request: &CategoryRequest, user_id: &Uuid) -> Result<Category, AppError> {
        let row = sqlx::query_as::<_, CategoryRow>(
            r#"
            INSERT INTO category (user_id, name, color, icon, parent_id, category_type)
            VALUES ($1, $2, $3, $4, $5, $6::text::category_type)
            RETURNING
                id,
                user_id,
                name,
                COALESCE(color, '') as color,
                COALESCE(icon, '') as icon,
                parent_id,
                category_type::text as category_type,
                created_at
            "#,
        )
        .bind(user_id)
        .bind(&request.name)
        .bind(&request.color)
        .bind(&request.icon)
        .bind(request.parent_id)
        .bind(request.category_type_to_db())
        .fetch_one(&self.pool)
        .await?;

        Ok(Category::from(row))
    }

    pub async fn get_category_by_id(&self, id: &Uuid, user_id: &Uuid) -> Result<Option<Category>, AppError> {
        let row = sqlx::query_as::<_, CategoryRow>(
            r#"
            SELECT
                id,
                user_id,
                name,
                COALESCE(color, '') as color,
                COALESCE(icon, '') as icon,
                parent_id,
                category_type::text as category_type,
                created_at
            FROM category
            WHERE id = $1 AND user_id = $2
            "#,
        )
        .bind(id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Category::from))
    }

    pub async fn list_categories(&self, params: &CursorParams, user_id: &Uuid) -> Result<Vec<Category>, AppError> {
        let rows = if let Some(cursor) = params.cursor {
            sqlx::query_as::<_, CategoryRow>(
                r#"
                SELECT
                    id,
                    user_id,
                    name,
                    COALESCE(color, '') as color,
                    COALESCE(icon, '') as icon,
                    parent_id,
                    category_type::text as category_type,
                    created_at
                FROM category
                WHERE user_id = $1
                    AND (created_at, id) < (SELECT created_at, id FROM category WHERE id = $2)
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
            sqlx::query_as::<_, CategoryRow>(
                r#"
                SELECT
                    id,
                    user_id,
                    name,
                    COALESCE(color, '') as color,
                    COALESCE(icon, '') as icon,
                    parent_id,
                    category_type::text as category_type,
                    created_at
                FROM category
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

        Ok(rows.into_iter().map(Category::from).collect())
    }

    pub async fn delete_category(&self, id: &Uuid, user_id: &Uuid) -> Result<(), AppError> {
        sqlx::query("DELETE FROM category WHERE id = $1 AND user_id = $2")
            .bind(id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn update_category(&self, id: &Uuid, request: &CategoryRequest, user_id: &Uuid) -> Result<Category, AppError> {
        let row = sqlx::query_as::<_, CategoryRow>(
            r#"
            UPDATE category
            SET name = $1, color = $2, icon = $3, parent_id = $4, category_type = $5::text::category_type
            WHERE id = $6 AND user_id = $7
            RETURNING
                id,
                user_id,
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
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(Category::from(row))
    }

    pub async fn list_categories_not_in_budget(&self, params: &CursorParams, user_id: &Uuid) -> Result<Vec<Category>, AppError> {
        let rows = if let Some(cursor) = params.cursor {
            sqlx::query_as::<_, CategoryRow>(
                r#"
                SELECT
                    c.id,
                    c.user_id,
                    c.name,
                    COALESCE(c.color, '') as color,
                    COALESCE(c.icon, '') as icon,
                    c.parent_id,
                    c.category_type::text as category_type,
                    c.created_at
                FROM category c
                LEFT JOIN budget_category bc ON c.id = bc.category_id
                WHERE bc.id IS NULL
                    AND c.category_type = 'Outgoing'
                    AND c.user_id = $1
                    AND (c.created_at, c.id) < (SELECT created_at, id FROM category WHERE id = $2)
                ORDER BY c.created_at DESC, c.id DESC
                LIMIT $3
                "#,
            )
            .bind(user_id)
            .bind(cursor)
            .bind(params.fetch_limit())
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, CategoryRow>(
                r#"
                SELECT
                    c.id,
                    c.user_id,
                    c.name,
                    COALESCE(c.color, '') as color,
                    COALESCE(c.icon, '') as icon,
                    c.parent_id,
                    c.category_type::text as category_type,
                    c.created_at
                FROM category c
                LEFT JOIN budget_category bc ON c.id = bc.category_id
                WHERE bc.id IS NULL
                    AND c.category_type = 'Outgoing'
                    AND c.user_id = $1
                ORDER BY c.created_at DESC, c.id DESC
                LIMIT $2
                "#,
            )
            .bind(user_id)
            .bind(params.fetch_limit())
            .fetch_all(&self.pool)
            .await?
        };

        Ok(rows.into_iter().map(Category::from).collect())
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
