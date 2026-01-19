use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::models::category::{Category, CategoryRequest, CategoryType};
use tokio_postgres::Row;
use uuid::Uuid;

#[async_trait::async_trait]
pub trait CategoryRepository {
    async fn create_category(&self, request: &CategoryRequest) -> Result<Category, AppError>;
    async fn get_category_by_id(&self, id: &Uuid) -> Result<Option<Category>, AppError>;
    async fn list_categories(&self) -> Result<Vec<Category>, AppError>;
    async fn delete_category(&self, id: &Uuid) -> Result<(), AppError>;
    async fn update_category(&self, id: &Uuid, request: &CategoryRequest) -> Result<Category, AppError>;
    async fn list_categories_not_in_budget(&self) -> Result<Vec<Category>, AppError>;
}

#[async_trait::async_trait]
impl<'a> CategoryRepository for PostgresRepository<'a> {
    async fn create_category(&self, request: &CategoryRequest) -> Result<Category, AppError> {
        let rows = self
            .client
            .query(
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
                &[&request.name, &request.color, &request.icon, &request.parent_id, &request.category_type_to_db()],
            )
            .await?;

        if let Some(row) = rows.first() {
            Ok(map_row_to_category(row))
        } else {
            Err(AppError::Db("Error mapping created category".to_string()))
        }
    }

    async fn get_category_by_id(&self, id: &Uuid) -> Result<Option<Category>, AppError> {
        let rows = self
            .client
            .query(
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
                &[id],
            )
            .await?;

        if let Some(row) = rows.first() {
            Ok(Some(map_row_to_category(row)))
        } else {
            Ok(None)
        }
    }

    async fn list_categories(&self) -> Result<Vec<Category>, AppError> {
        let rows = self
            .client
            .query(
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
                &[],
            )
            .await?;

        Ok(rows.into_iter().map(|r| map_row_to_category(&r)).collect())
    }

    async fn delete_category(&self, id: &Uuid) -> Result<(), AppError> {
        self.client
            .execute(
                r#"
            DELETE FROM category
            WHERE id = $1
            "#,
                &[id],
            )
            .await?;
        Ok(())
    }

    async fn update_category(&self, id: &Uuid, request: &CategoryRequest) -> Result<Category, AppError> {
        let rows = self
            .client
            .query(
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
                &[
                    &request.name,
                    &request.color,
                    &request.icon,
                    &request.parent_id,
                    &request.category_type_to_db(),
                    &id,
                ],
            )
            .await?;

        if let Some(row) = rows.first() {
            Ok(map_row_to_category(row))
        } else {
            Err(AppError::NotFound("Category not found".to_string()))
        }
    }

    async fn list_categories_not_in_budget(&self) -> Result<Vec<Category>, AppError> {
        let rows = self
            .client
            .query(
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
                &[],
            )
            .await?;

        Ok(rows.into_iter().map(|r| map_row_to_category(&r)).collect())
    }
}

fn map_row_to_category(row: &Row) -> Category {
    Category {
        id: row.get("id"),
        name: row.get("name"),
        color: row.get("color"),
        icon: row.get("icon"),
        parent_id: row.get("parent_id"),
        category_type: category_type_from_db(row.get::<_, &str>("category_type")),
        created_at: row.get("created_at"),
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
