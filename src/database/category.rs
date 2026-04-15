use crate::crypto::Dek;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::categories::{CreateCategoryRequest, CreateTargetRequest, UpdateCategoryRequest, UpdateTargetRequest};
use crate::error::app_error::AppError;
use crate::models::category::{Category, CategoryBehavior, CategoryType};
use uuid::Uuid;

const CATEGORY_COLUMNS: &str = "id, category_type, behavior, parent_id, is_system, is_archived, name_enc, color_enc, icon_enc, description_enc";

impl PostgresRepository {
    /// Encrypt the request and insert a new category. Name uniqueness
    /// is enforced in Rust inside a tx that locks the users row.
    pub async fn create_category(&self, request: &CreateCategoryRequest, user_id: &Uuid, dek: &Dek) -> Result<Category, AppError> {
        let mut tx = self.pool.begin().await?;

        lock_user_row(&mut tx, user_id).await?;
        check_category_name_unique(&mut tx, dek, user_id, &request.name, None).await?;

        let name_enc = dek.encrypt_string(&request.name)?;
        let color_enc = request.color.as_deref().map(|c| dek.encrypt_string(c)).transpose()?;
        let icon_enc = dek.encrypt_string(&request.icon)?;
        let description_enc = request.description.as_deref().map(|d| dek.encrypt_string(d)).transpose()?;

        let category: Category = sqlx::query_as(&format!(
            r#"
INSERT INTO category (
    id, user_id, category_type, behavior, parent_id, is_system, is_archived,
    name_enc, color_enc, icon_enc, description_enc
) VALUES (
    gen_random_uuid(), $1, $2, $3, $4, false, false,
    $5, $6, $7, $8
)
RETURNING {CATEGORY_COLUMNS}
"#,
        ))
        .bind(user_id)
        .bind::<CategoryType>(request.category_type.into())
        .bind(request.behavior.map(CategoryBehavior::from))
        .bind(request.parent_id)
        .bind(&name_enc)
        .bind(color_enc.as_deref())
        .bind(&icon_enc)
        .bind(description_enc.as_deref())
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(category)
    }

    #[allow(dead_code)]
    pub async fn get_category_by_id(&self, id: &Uuid, user_id: &Uuid) -> Result<Option<Category>, AppError> {
        let category = sqlx::query_as::<_, Category>(&format!("SELECT {CATEGORY_COLUMNS} FROM category WHERE id = $1 AND user_id = $2",))
            .bind(id)
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(category)
    }

    pub async fn list_categories(&self, user_id: &Uuid) -> Result<Vec<Category>, AppError> {
        let categories = sqlx::query_as::<_, Category>(&format!("SELECT {CATEGORY_COLUMNS} FROM category WHERE user_id = $1 ORDER BY id",))
            .bind(user_id)
            .fetch_all(&self.pool)
            .await?;
        Ok(categories)
    }

    pub async fn update_category(&self, id: &Uuid, request: &UpdateCategoryRequest, user_id: &Uuid, dek: &Dek) -> Result<Category, AppError> {
        let mut tx = self.pool.begin().await?;

        lock_user_row(&mut tx, user_id).await?;
        check_category_name_unique(&mut tx, dek, user_id, &request.name, Some(id)).await?;

        let name_enc = dek.encrypt_string(&request.name)?;
        let color_enc = request.color.as_deref().map(|c| dek.encrypt_string(c)).transpose()?;
        let icon_enc = dek.encrypt_string(&request.icon)?;
        let description_enc = request.description.as_deref().map(|d| dek.encrypt_string(d)).transpose()?;

        let category: Category = sqlx::query_as(&format!(
            r#"
UPDATE category
SET category_type = $1,
    behavior = $2,
    parent_id = $3,
    name_enc = $4,
    color_enc = $5,
    icon_enc = $6,
    description_enc = $7
WHERE id = $8 AND user_id = $9
RETURNING {CATEGORY_COLUMNS}
"#,
        ))
        .bind::<CategoryType>(request.category_type.into())
        .bind(request.behavior.map(CategoryBehavior::from))
        .bind(request.parent_id)
        .bind(&name_enc)
        .bind(color_enc.as_deref())
        .bind(&icon_enc)
        .bind(description_enc.as_deref())
        .bind(id)
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Category not found".to_string()))?;

        tx.commit().await?;
        Ok(category)
    }

    pub async fn delete_category(&self, id: &Uuid, user_id: &Uuid) -> Result<(), AppError> {
        let result = sqlx::query("DELETE FROM category WHERE id = $1 AND user_id = $2 AND NOT is_system")
            .bind(id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("Category not found".to_string()));
        }
        Ok(())
    }

    pub async fn archive_category(&self, id: &Uuid, user_id: &Uuid) -> Result<(), AppError> {
        let result = sqlx::query("UPDATE category SET is_archived = true WHERE id = $1 AND user_id = $2 AND NOT is_system")
            .bind(id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("Category not found".to_string()));
        }
        Ok(())
    }

    pub async fn unarchive_category(&self, id: &Uuid, user_id: &Uuid) -> Result<(), AppError> {
        let result = sqlx::query("UPDATE category SET is_archived = false WHERE id = $1 AND user_id = $2")
            .bind(id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("Category not found".to_string()));
        }
        Ok(())
    }

    // ===== Targets (budget_category) =====

    pub async fn list_targets(&self, user_id: &Uuid) -> Result<Vec<(Uuid, Uuid, bool, Vec<u8>)>, AppError> {
        let rows: Vec<(Uuid, Uuid, bool, Vec<u8>)> = sqlx::query_as(
            r#"
SELECT bc.id, bc.category_id, bc.is_excluded, bc.budgeted_value_enc
FROM budget_category bc
WHERE bc.user_id = $1
ORDER BY bc.id
"#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn create_target(&self, request: &CreateTargetRequest, user_id: &Uuid, dek: &Dek) -> Result<(Uuid, Uuid, bool, Vec<u8>), AppError> {
        // Ensure the category belongs to the user and there isn't
        // already a target row for it (budget_category is 1:1 with
        // category in v2).
        let exists: Option<(Uuid,)> = sqlx::query_as("SELECT id FROM category WHERE id = $1 AND user_id = $2")
            .bind(request.category_id)
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await?;
        if exists.is_none() {
            return Err(AppError::NotFound("Category not found".to_string()));
        }

        let value_enc = dek.encrypt_i64(request.value)?;

        let row: (Uuid, Uuid, bool, Vec<u8>) = sqlx::query_as(
            r#"
INSERT INTO budget_category (id, user_id, category_id, is_excluded, budgeted_value_enc)
VALUES (gen_random_uuid(), $1, $2, false, $3)
RETURNING id, category_id, is_excluded, budgeted_value_enc
"#,
        )
        .bind(user_id)
        .bind(request.category_id)
        .bind(&value_enc)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn update_target(
        &self,
        target_id: &Uuid,
        request: &UpdateTargetRequest,
        user_id: &Uuid,
        dek: &Dek,
    ) -> Result<(Uuid, Uuid, bool, Vec<u8>), AppError> {
        let value_enc = dek.encrypt_i64(request.value)?;
        let row = sqlx::query_as::<_, (Uuid, Uuid, bool, Vec<u8>)>(
            r#"
UPDATE budget_category
SET budgeted_value_enc = $1
WHERE id = $2 AND user_id = $3
RETURNING id, category_id, is_excluded, budgeted_value_enc
"#,
        )
        .bind(&value_enc)
        .bind(target_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Target not found".to_string()))?;
        Ok(row)
    }

    pub async fn toggle_target_excluded(&self, target_id: &Uuid, user_id: &Uuid) -> Result<(Uuid, Uuid, bool, Vec<u8>), AppError> {
        let row = sqlx::query_as::<_, (Uuid, Uuid, bool, Vec<u8>)>(
            r#"
UPDATE budget_category
SET is_excluded = NOT is_excluded
WHERE id = $1 AND user_id = $2
RETURNING id, category_id, is_excluded, budgeted_value_enc
"#,
        )
        .bind(target_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Target not found".to_string()))?;
        Ok(row)
    }
}

async fn lock_user_row(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, user_id: &Uuid) -> Result<(), AppError> {
    sqlx::query("SELECT 1 FROM users WHERE id = $1 FOR UPDATE")
        .bind(user_id)
        .fetch_one(&mut **tx)
        .await?;
    Ok(())
}

async fn check_category_name_unique(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    dek: &Dek,
    user_id: &Uuid,
    candidate: &str,
    exclude_id: Option<&Uuid>,
) -> Result<(), AppError> {
    let rows: Vec<(Uuid, Vec<u8>)> = sqlx::query_as("SELECT id, name_enc FROM category WHERE user_id = $1")
        .bind(user_id)
        .fetch_all(&mut **tx)
        .await?;

    let candidate_lower = candidate.to_lowercase();
    for (row_id, name_enc) in rows {
        if exclude_id.is_some_and(|id| *id == row_id) {
            continue;
        }
        let existing = dek.decrypt_string(&name_enc)?;
        if existing.to_lowercase() == candidate_lower {
            return Err(AppError::Conflict(format!("A category named '{}' already exists", candidate)));
        }
    }
    Ok(())
}
