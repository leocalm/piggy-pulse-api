use crate::crypto::Dek;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::vendors::{CreateVendorRequest, UpdateVendorRequest};
use crate::error::app_error::AppError;
use crate::models::vendor::Vendor;
use uuid::Uuid;

const VENDOR_COLUMNS: &str = "id, archived, name_enc, description_enc";

impl PostgresRepository {
    pub async fn create_vendor(&self, request: &CreateVendorRequest, user_id: &Uuid, dek: &Dek) -> Result<Vendor, AppError> {
        let mut tx = self.pool.begin().await?;

        lock_user_row(&mut tx, user_id).await?;
        check_vendor_name_unique(&mut tx, dek, user_id, &request.name, None).await?;

        let name_enc = dek.encrypt_string(&request.name)?;
        let description_enc = request.description.as_deref().map(|d| dek.encrypt_string(d)).transpose()?;

        let vendor: Vendor = sqlx::query_as(&format!(
            r#"
INSERT INTO vendor (id, user_id, archived, name_enc, description_enc)
VALUES (gen_random_uuid(), $1, false, $2, $3)
RETURNING {VENDOR_COLUMNS}
"#,
        ))
        .bind(user_id)
        .bind(&name_enc)
        .bind(description_enc.as_deref())
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(vendor)
    }

    #[allow(dead_code)]
    pub async fn get_vendor_by_id(&self, id: &Uuid, user_id: &Uuid) -> Result<Option<Vendor>, AppError> {
        let vendor = sqlx::query_as::<_, Vendor>(&format!("SELECT {VENDOR_COLUMNS} FROM vendor WHERE id = $1 AND user_id = $2",))
            .bind(id)
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(vendor)
    }

    pub async fn list_vendors(&self, user_id: &Uuid) -> Result<Vec<Vendor>, AppError> {
        let vendors = sqlx::query_as::<_, Vendor>(&format!("SELECT {VENDOR_COLUMNS} FROM vendor WHERE user_id = $1 ORDER BY id",))
            .bind(user_id)
            .fetch_all(&self.pool)
            .await?;
        Ok(vendors)
    }

    pub async fn update_vendor(&self, id: &Uuid, request: &UpdateVendorRequest, user_id: &Uuid, dek: &Dek) -> Result<Vendor, AppError> {
        let mut tx = self.pool.begin().await?;

        lock_user_row(&mut tx, user_id).await?;
        check_vendor_name_unique(&mut tx, dek, user_id, &request.name, Some(id)).await?;

        let name_enc = dek.encrypt_string(&request.name)?;
        let description_enc = request.description.as_deref().map(|d| dek.encrypt_string(d)).transpose()?;

        let vendor: Vendor = sqlx::query_as(&format!(
            r#"
UPDATE vendor
SET name_enc = $1, description_enc = $2
WHERE id = $3 AND user_id = $4
RETURNING {VENDOR_COLUMNS}
"#,
        ))
        .bind(&name_enc)
        .bind(description_enc.as_deref())
        .bind(id)
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Vendor not found".to_string()))?;

        tx.commit().await?;
        Ok(vendor)
    }

    pub async fn delete_vendor(&self, id: &Uuid, user_id: &Uuid) -> Result<(), AppError> {
        let result = sqlx::query("DELETE FROM vendor WHERE id = $1 AND user_id = $2")
            .bind(id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("Vendor not found".to_string()));
        }
        Ok(())
    }

    pub async fn archive_vendor(&self, id: &Uuid, user_id: &Uuid) -> Result<(), AppError> {
        let result = sqlx::query("UPDATE vendor SET archived = true WHERE id = $1 AND user_id = $2")
            .bind(id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("Vendor not found".to_string()));
        }
        Ok(())
    }

    pub async fn unarchive_vendor(&self, id: &Uuid, user_id: &Uuid) -> Result<(), AppError> {
        let result = sqlx::query("UPDATE vendor SET archived = false WHERE id = $1 AND user_id = $2")
            .bind(id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("Vendor not found".to_string()));
        }
        Ok(())
    }
}

async fn lock_user_row(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, user_id: &Uuid) -> Result<(), AppError> {
    sqlx::query("SELECT 1 FROM users WHERE id = $1 FOR UPDATE")
        .bind(user_id)
        .fetch_one(&mut **tx)
        .await?;
    Ok(())
}

async fn check_vendor_name_unique(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    dek: &Dek,
    user_id: &Uuid,
    candidate: &str,
    exclude_id: Option<&Uuid>,
) -> Result<(), AppError> {
    let rows: Vec<(Uuid, Vec<u8>)> = sqlx::query_as("SELECT id, name_enc FROM vendor WHERE user_id = $1")
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
            return Err(AppError::Conflict(format!("A vendor named '{}' already exists", candidate)));
        }
    }
    Ok(())
}
