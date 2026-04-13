use crate::database::postgres_repository::{PostgresRepository, is_unique_violation};
use crate::error::app_error::AppError;
use crate::models::category::{Category, CategoryRequest, CategoryType};
use uuid::Uuid;

// Intermediate struct for sqlx query results with category_type as text
#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct CategoryRow {
    id: Uuid,
    name: String,
    color: String,
    icon: String,
    parent_id: Option<Uuid>,
    category_type: String,
    is_archived: bool,
    description: Option<String>,
    is_system: bool,
    behavior: Option<String>,
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
            is_archived: row.is_archived,
            description: row.description,
            is_system: row.is_system,
            behavior: row.behavior.as_deref().and_then(crate::models::category::category_behavior_from_db),
        }
    }
}

impl PostgresRepository {
    pub async fn create_category(&self, request: &CategoryRequest, user_id: &Uuid) -> Result<Category, AppError> {
        // Validate max depth = 1 (cannot set parent_id to a category that already has a parent)
        if let Some(parent_id) = request.parent_id {
            let parent: Option<CategoryRow> = sqlx::query_as(
                r#"
                SELECT
                    id,
                    name,
                    COALESCE(color, '') as color,
                    COALESCE(icon, '') as icon,
                    parent_id,
                    category_type::text as category_type,
                    is_archived,
                    description,
                    is_system,
                    behavior::text as behavior
                FROM category
                WHERE id = $1 AND user_id = $2
                "#,
            )
            .bind(parent_id)
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await?;

            if let Some(parent) = parent {
                if parent.parent_id.is_some() {
                    return Err(AppError::BadRequest(
                        "Cannot create a subcategory under another subcategory. Maximum depth is 1.".to_string(),
                    ));
                }
                // Verify type matches parent
                let parent_type = category_type_from_db(&parent.category_type);
                if parent_type != request.category_type {
                    return Err(AppError::BadRequest("Subcategory must have the same type as its parent.".to_string()));
                }
            } else {
                return Err(AppError::NotFound("Parent category not found".to_string()));
            }
        }

        let name_exists: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS (
                SELECT 1
                FROM category
                WHERE user_id = $1 AND name = $2
            )
            "#,
        )
        .bind(user_id)
        .bind(&request.name)
        .fetch_one(&self.pool)
        .await?;

        if name_exists {
            return Err(AppError::BadRequest("Category name already exists".to_string()));
        }

        let row = sqlx::query_as::<_, CategoryRow>(
            r#"
            INSERT INTO category (user_id, name, color, icon, parent_id, category_type, is_archived, description, behavior)
            VALUES ($1, $2, $3, $4, $5, $6::text::category_type, FALSE, $7, $8::text::category_behavior)
            RETURNING
                id,
                name,
                COALESCE(color, '') as color,
                COALESCE(icon, '') as icon,
                parent_id,
                category_type::text as category_type,
                is_archived,
                description,
                is_system,
                behavior::text as behavior
            "#,
        )
        .bind(user_id)
        .bind(&request.name)
        .bind(&request.color)
        .bind(&request.icon)
        .bind(request.parent_id)
        .bind(request.category_type_to_db())
        .bind(&request.description)
        .bind(request.behavior.as_deref())
        .fetch_one(&self.pool)
        .await;

        let row = match row {
            Ok(row) => row,
            Err(err) if is_unique_violation(&err) => {
                return Err(AppError::BadRequest("Category name already exists".to_string()));
            }
            Err(err) => return Err(err.into()),
        };

        Ok(Category::from(row))
    }

    /// Create a category and upsert its target atomically within a single transaction.
    pub async fn create_category_with_target(&self, request: &CategoryRequest, target_value: i64, user_id: &Uuid) -> Result<Category, AppError> {
        let mut tx = self.pool.begin().await?;

        // Validate parent (same logic as create_category, but against tx)
        if let Some(parent_id) = request.parent_id {
            let parent: Option<CategoryRow> = sqlx::query_as(
                r#"
                SELECT id, name, COALESCE(color, '') as color, COALESCE(icon, '') as icon,
                       parent_id, category_type::text as category_type, is_archived,
                       description, is_system, behavior::text as behavior
                FROM category WHERE id = $1 AND user_id = $2
                "#,
            )
            .bind(parent_id)
            .bind(user_id)
            .fetch_optional(&mut *tx)
            .await?;

            if let Some(parent) = parent {
                if parent.parent_id.is_some() {
                    return Err(AppError::BadRequest(
                        "Cannot create a subcategory under another subcategory. Maximum depth is 1.".to_string(),
                    ));
                }
                let parent_type = category_type_from_db(&parent.category_type);
                if parent_type != request.category_type {
                    return Err(AppError::BadRequest("Subcategory must have the same type as its parent.".to_string()));
                }
            } else {
                return Err(AppError::NotFound("Parent category not found".to_string()));
            }
        }

        let row = sqlx::query_as::<_, CategoryRow>(
            r#"
            INSERT INTO category (user_id, name, color, icon, parent_id, category_type, is_archived, description, behavior)
            VALUES ($1, $2, $3, $4, $5, $6::text::category_type, FALSE, $7, $8::text::category_behavior)
            RETURNING id, name, COALESCE(color, '') as color, COALESCE(icon, '') as icon,
                      parent_id, category_type::text as category_type, is_archived,
                      description, is_system, behavior::text as behavior
            "#,
        )
        .bind(user_id)
        .bind(&request.name)
        .bind(&request.color)
        .bind(&request.icon)
        .bind(request.parent_id)
        .bind(request.category_type_to_db())
        .bind(&request.description)
        .bind(request.behavior.as_deref())
        .fetch_one(&mut *tx)
        .await;

        let row = match row {
            Ok(row) => row,
            Err(err) if is_unique_violation(&err) => {
                return Err(AppError::BadRequest("Category name already exists".to_string()));
            }
            Err(err) => return Err(err.into()),
        };

        let category = Category::from(row);

        // Upsert target within same transaction
        sqlx::query(
            r#"
            INSERT INTO budget_category (user_id, category_id, budgeted_value, is_excluded)
            VALUES ($1, $2, $3, FALSE)
            ON CONFLICT (user_id, category_id) DO UPDATE SET budgeted_value = EXCLUDED.budgeted_value, is_excluded = FALSE
            "#,
        )
        .bind(user_id)
        .bind(category.id)
        .bind(i32::try_from(target_value).map_err(|_| AppError::BadRequest("Target value out of range".to_string()))?)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(category)
    }

    /// Update a category and upsert its target atomically within a single transaction.
    pub async fn update_category_with_target(&self, id: &Uuid, request: &CategoryRequest, target_value: i64, user_id: &Uuid) -> Result<Category, AppError> {
        let mut tx = self.pool.begin().await?;

        // Guard: system categories cannot be updated
        let is_system: bool = sqlx::query_scalar("SELECT is_system FROM category WHERE id = $1 AND user_id = $2")
            .bind(id)
            .bind(user_id)
            .fetch_optional(&mut *tx)
            .await?
            .unwrap_or(false);
        if is_system {
            return Err(AppError::BadRequest("System categories cannot be modified.".to_string()));
        }

        // Validate parent_id
        if let Some(parent_id) = request.parent_id {
            if parent_id == *id {
                return Err(AppError::BadRequest("A category cannot be its own parent.".to_string()));
            }
            let parent: Option<CategoryRow> = sqlx::query_as(
                r#"
                SELECT id, name, COALESCE(color, '') as color, COALESCE(icon, '') as icon,
                       parent_id, category_type::text as category_type, is_archived,
                       description, is_system, behavior::text as behavior
                FROM category WHERE id = $1 AND user_id = $2
                "#,
            )
            .bind(parent_id)
            .bind(user_id)
            .fetch_optional(&mut *tx)
            .await?;

            if let Some(parent) = parent {
                if parent.parent_id.is_some() {
                    return Err(AppError::BadRequest(
                        "Cannot create a subcategory under another subcategory. Maximum depth is 1.".to_string(),
                    ));
                }
                let parent_type = category_type_from_db(&parent.category_type);
                if parent_type != request.category_type {
                    return Err(AppError::BadRequest("Subcategory must have the same type as its parent.".to_string()));
                }
                if parent.is_archived {
                    return Err(AppError::BadRequest("Cannot set parent to an archived category.".to_string()));
                }
            } else {
                return Err(AppError::NotFound("Parent category not found".to_string()));
            }
        }

        // Pre-check name uniqueness so that the duplicate-name case returns a
        // clean 400 even before the UPDATE hits the `(user_id, name)` unique
        // constraint (which we also map defensively below).
        let name_exists: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS (
                SELECT 1
                FROM category
                WHERE user_id = $1 AND name = $2 AND id <> $3
            )
            "#,
        )
        .bind(user_id)
        .bind(&request.name)
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;

        if name_exists {
            return Err(AppError::BadRequest("Category name already exists".to_string()));
        }

        let row_result = sqlx::query_as::<_, CategoryRow>(
            r#"
            UPDATE category
            SET name = $1, color = $2, icon = $3, parent_id = $4, category_type = $5::text::category_type,
                description = $6, behavior = $9::text::category_behavior
            WHERE id = $7 AND user_id = $8
            RETURNING id, name, COALESCE(color, '') as color, COALESCE(icon, '') as icon,
                      parent_id, category_type::text as category_type, is_archived,
                      description, is_system, behavior::text as behavior
            "#,
        )
        .bind(&request.name)
        .bind(&request.color)
        .bind(&request.icon)
        .bind(request.parent_id)
        .bind(request.category_type_to_db())
        .bind(&request.description)
        .bind(id)
        .bind(user_id)
        .bind(request.behavior.as_deref())
        .fetch_optional(&mut *tx)
        .await;

        let row = match row_result {
            Ok(Some(row)) => row,
            Ok(None) => return Err(AppError::NotFound("Category not found".to_string())),
            Err(err) if is_unique_violation(&err) => {
                return Err(AppError::BadRequest("Category name already exists".to_string()));
            }
            Err(err) => return Err(err.into()),
        };

        let category = Category::from(row);

        // Upsert target within same transaction
        sqlx::query(
            r#"
            INSERT INTO budget_category (user_id, category_id, budgeted_value, is_excluded)
            VALUES ($1, $2, $3, FALSE)
            ON CONFLICT (user_id, category_id) DO UPDATE SET budgeted_value = EXCLUDED.budgeted_value, is_excluded = FALSE
            "#,
        )
        .bind(user_id)
        .bind(id)
        .bind(i32::try_from(target_value).map_err(|_| AppError::BadRequest("Target value out of range".to_string()))?)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(category)
    }

    pub async fn get_category_by_id(&self, id: &Uuid, user_id: &Uuid) -> Result<Option<Category>, AppError> {
        let row = sqlx::query_as::<_, CategoryRow>(
            r#"
            SELECT
                id,
                name,
                COALESCE(color, '') as color,
                COALESCE(icon, '') as icon,
                parent_id,
                category_type::text as category_type,
                is_archived,
                description,
                is_system,
                behavior::text as behavior
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

    pub async fn delete_category(&self, id: &Uuid, user_id: &Uuid) -> Result<(), AppError> {
        // Guard: system categories cannot be deleted
        let is_system: bool = sqlx::query_scalar("SELECT is_system FROM category WHERE id = $1 AND user_id = $2")
            .bind(id)
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await?
            .unwrap_or(false);
        if is_system {
            return Err(AppError::BadRequest("System categories cannot be deleted.".to_string()));
        }

        // Check for transactions
        let transaction_count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)::bigint
            FROM transaction
            WHERE category_id = $1 AND user_id = $2
            "#,
        )
        .bind(id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        if transaction_count > 0 {
            return Err(AppError::BadRequest(
                "Cannot delete category with existing transactions. Archive it instead.".to_string(),
            ));
        }

        // Check for children
        let children_count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)::bigint
            FROM category
            WHERE parent_id = $1 AND user_id = $2
            "#,
        )
        .bind(id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        if children_count > 0 {
            return Err(AppError::BadRequest(
                "Cannot delete category with subcategories. Delete or archive subcategories first.".to_string(),
            ));
        }

        sqlx::query("DELETE FROM category WHERE id = $1 AND user_id = $2")
            .bind(id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn update_category(&self, id: &Uuid, request: &CategoryRequest, user_id: &Uuid) -> Result<Category, AppError> {
        // Guard: system categories cannot be updated
        let is_system: bool = sqlx::query_scalar("SELECT is_system FROM category WHERE id = $1 AND user_id = $2")
            .bind(id)
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await?
            .unwrap_or(false);
        if is_system {
            return Err(AppError::BadRequest("System categories cannot be modified.".to_string()));
        }

        // Validate parent_id if being set
        if let Some(parent_id) = request.parent_id {
            // Prevent setting parent_id to self
            if parent_id == *id {
                return Err(AppError::BadRequest("A category cannot be its own parent.".to_string()));
            }

            let parent: Option<CategoryRow> = sqlx::query_as(
                r#"
                SELECT
                    id,
                    name,
                    COALESCE(color, '') as color,
                    COALESCE(icon, '') as icon,
                    parent_id,
                    category_type::text as category_type,
                    is_archived,
                    description,
                    is_system,
                    behavior::text as behavior
                FROM category
                WHERE id = $1 AND user_id = $2
                "#,
            )
            .bind(parent_id)
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await?;

            if let Some(parent) = parent {
                // Max depth = 1: parent cannot already have a parent
                if parent.parent_id.is_some() {
                    return Err(AppError::BadRequest(
                        "Cannot create a subcategory under another subcategory. Maximum depth is 1.".to_string(),
                    ));
                }
                // Parent must be same type
                let parent_type = category_type_from_db(&parent.category_type);
                if parent_type != request.category_type {
                    return Err(AppError::BadRequest("Subcategory must have the same type as its parent.".to_string()));
                }
                // Parent cannot be archived
                if parent.is_archived {
                    return Err(AppError::BadRequest("Cannot set parent to an archived category.".to_string()));
                }
            } else {
                return Err(AppError::NotFound("Parent category not found".to_string()));
            }
        }

        let name_exists: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS (
                SELECT 1
                FROM category
                WHERE user_id = $1 AND name = $2 AND id <> $3
            )
            "#,
        )
        .bind(user_id)
        .bind(&request.name)
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        if name_exists {
            return Err(AppError::BadRequest("Category name already exists".to_string()));
        }

        let row = sqlx::query_as::<_, CategoryRow>(
            r#"
            UPDATE category
            SET name = $1, color = $2, icon = $3, parent_id = $4, category_type = $5::text::category_type, description = $6,
                behavior = $9::text::category_behavior
            WHERE id = $7 AND user_id = $8
            RETURNING
                id,
                name,
                COALESCE(color, '') as color,
                COALESCE(icon, '') as icon,
                parent_id,
                category_type::text as category_type,
                is_archived,
                description,
                is_system,
                behavior::text as behavior
            "#,
        )
        .bind(&request.name)
        .bind(&request.color)
        .bind(&request.icon)
        .bind(request.parent_id)
        .bind(request.category_type_to_db())
        .bind(&request.description)
        .bind(id)
        .bind(user_id)
        .bind(request.behavior.as_deref())
        .fetch_one(&self.pool)
        .await;

        let row = match row {
            Ok(row) => row,
            Err(err) if is_unique_violation(&err) => {
                return Err(AppError::BadRequest("Category name already exists".to_string()));
            }
            Err(err) => return Err(err.into()),
        };

        Ok(Category::from(row))
    }

    pub async fn list_all_categories(&self, user_id: &Uuid) -> Result<Vec<Category>, AppError> {
        let rows = sqlx::query_as::<_, CategoryRow>(
            r#"
            SELECT
                c.id,
                c.name,
                COALESCE(c.color, '') as color,
                COALESCE(c.icon, '') as icon,
                c.parent_id,
                c.category_type::text as category_type,
                c.is_archived,
                c.description,
                c.is_system,
                c.behavior::text as behavior
            FROM category c
            WHERE c.user_id = $1
              AND (c.is_system = FALSE OR c.category_type = 'Transfer')
            ORDER BY c.created_at DESC, c.id DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Category::from).collect())
    }

    /// Archive a category (soft delete)
    pub async fn archive_category(&self, id: &Uuid, user_id: &Uuid) -> Result<Category, AppError> {
        // Guard: system categories cannot be archived
        let is_system: bool = sqlx::query_scalar("SELECT is_system FROM category WHERE id = $1 AND user_id = $2")
            .bind(id)
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await?
            .unwrap_or(false);
        if is_system {
            return Err(AppError::BadRequest("System categories cannot be archived.".to_string()));
        }

        // Check for active children
        let active_children_count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)::bigint
            FROM category
            WHERE parent_id = $1 AND user_id = $2 AND is_archived = FALSE
            "#,
        )
        .bind(id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        if active_children_count > 0 {
            return Err(AppError::BadRequest(
                "Cannot archive category with active subcategories. Archive subcategories first.".to_string(),
            ));
        }

        let row = sqlx::query_as::<_, CategoryRow>(
            r#"
            UPDATE category
            SET is_archived = TRUE
            WHERE id = $1 AND user_id = $2
            RETURNING
                id,
                name,
                COALESCE(color, '') as color,
                COALESCE(icon, '') as icon,
                parent_id,
                category_type::text as category_type,
                is_archived,
                description,
                is_system,
                behavior::text as behavior
            "#,
        )
        .bind(id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(Category::from(row))
    }

    /// Create the system Transfer category for a user
    pub async fn create_system_transfer_category(&self, user_id: &Uuid) -> Result<Category, AppError> {
        let row = sqlx::query_as::<_, CategoryRow>(
            r#"
            INSERT INTO category (user_id, name, color, icon, category_type, is_system)
            VALUES ($1, 'Transfer', '#868E96', '↔', 'Transfer'::category_type, TRUE)
            ON CONFLICT DO NOTHING
            RETURNING
                id,
                name,
                COALESCE(color, '') as color,
                COALESCE(icon, '') as icon,
                parent_id,
                category_type::text as category_type,
                is_archived,
                description,
                is_system,
                behavior::text as behavior
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(r) => Ok(Category::from(r)),
            None => self.get_transfer_category(user_id).await,
        }
    }

    /// Get the system Transfer category for a user
    pub async fn get_transfer_category(&self, user_id: &Uuid) -> Result<Category, AppError> {
        let row = sqlx::query_as::<_, CategoryRow>(
            r#"
            SELECT
                id,
                name,
                COALESCE(color, '') as color,
                COALESCE(icon, '') as icon,
                parent_id,
                category_type::text as category_type,
                is_archived,
                description,
                is_system,
                behavior::text as behavior
            FROM category
            WHERE user_id = $1 AND is_system = TRUE AND category_type = 'Transfer'
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(r) => Ok(Category::from(r)),
            None => Err(AppError::NotFound("System Transfer category not found".to_string())),
        }
    }

    /// Restore an archived category
    pub async fn restore_category(&self, id: &Uuid, user_id: &Uuid) -> Result<Category, AppError> {
        // Check if parent is archived (if has parent)
        let parent_archived: bool = sqlx::query_scalar(
            r#"
            SELECT COALESCE(
                (SELECT is_archived FROM category c2 WHERE c2.id = c.parent_id),
                FALSE
            )
            FROM category c
            WHERE c.id = $1 AND c.user_id = $2
            "#,
        )
        .bind(id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        if parent_archived {
            return Err(AppError::BadRequest(
                "Cannot restore subcategory when parent is archived. Restore parent first.".to_string(),
            ));
        }

        let row = sqlx::query_as::<_, CategoryRow>(
            r#"
            UPDATE category
            SET is_archived = FALSE
            WHERE id = $1 AND user_id = $2
            RETURNING
                id,
                name,
                COALESCE(color, '') as color,
                COALESCE(icon, '') as icon,
                parent_id,
                category_type::text as category_type,
                is_archived,
                description,
                is_system,
                behavior::text as behavior
            "#,
        )
        .bind(id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(Category::from(row))
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
            description: None,
            behavior: None,
        };
        assert_eq!(request.category_type_to_db(), "Incoming");

        let request_outgoing = CategoryRequest {
            name: "Test".to_string(),
            color: "#000000".to_string(),
            icon: "icon".to_string(),
            parent_id: None,
            category_type: CategoryType::Outgoing,
            description: None,
            behavior: None,
        };
        assert_eq!(request_outgoing.category_type_to_db(), "Outgoing");

        let request_transfer = CategoryRequest {
            name: "Test".to_string(),
            color: "#000000".to_string(),
            icon: "icon".to_string(),
            parent_id: None,
            category_type: CategoryType::Transfer,
            description: None,
            behavior: None,
        };
        assert_eq!(request_transfer.category_type_to_db(), "Transfer");
    }
}
