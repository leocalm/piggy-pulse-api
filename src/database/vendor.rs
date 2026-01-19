use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::models::vendor::{Vendor, VendorRequest};
use tokio_postgres::Row;
use uuid::Uuid;

#[async_trait::async_trait]
pub trait VendorRepository {
    async fn create_vendor(&self, request: &VendorRequest) -> Result<Vendor, AppError>;
    async fn get_vendor_by_id(&self, id: &Uuid) -> Result<Option<Vendor>, AppError>;
    async fn list_vendors(&self) -> Result<Vec<Vendor>, AppError>;
    async fn delete_vendor(&self, id: &Uuid) -> Result<(), AppError>;
    async fn update_vendor(&self, id: &Uuid, request: &VendorRequest) -> Result<Vendor, AppError>;
}

#[async_trait::async_trait]
impl<'a> VendorRepository for PostgresRepository<'a> {
    async fn create_vendor(&self, request: &VendorRequest) -> Result<Vendor, AppError> {
        let rows = self
            .client
            .query(
                r#"
            INSERT INTO vendor (name)
            VALUES ($1)
            RETURNING id, name, created_at
            "#,
                &[&request.name],
            )
            .await?;

        if let Some(row) = rows.first() {
            Ok(map_row_to_vendor(row))
        } else {
            Err(AppError::Db("Error mapping created vendor".to_string()))
        }
    }

    async fn get_vendor_by_id(&self, id: &Uuid) -> Result<Option<Vendor>, AppError> {
        let rows = self
            .client
            .query(
                r#"
            SELECT id, name, created_at
            FROM vendor
            WHERE id = $1
            "#,
                &[id],
            )
            .await?;

        if let Some(row) = rows.first() {
            Ok(Some(map_row_to_vendor(row)))
        } else {
            Ok(None)
        }
    }

    async fn list_vendors(&self) -> Result<Vec<Vendor>, AppError> {
        let rows = self
            .client
            .query(
                r#"
            SELECT id, name, created_at
            FROM vendor
            ORDER BY created_at DESC
            "#,
                &[],
            )
            .await?;

        Ok(rows.into_iter().map(|r| map_row_to_vendor(&r)).collect())
    }

    async fn delete_vendor(&self, id: &Uuid) -> Result<(), AppError> {
        self.client
            .execute(
                r#"
            DELETE FROM vendor
            WHERE id = $1
            "#,
                &[id],
            )
            .await?;
        Ok(())
    }

    async fn update_vendor(&self, id: &Uuid, request: &VendorRequest) -> Result<Vendor, AppError> {
        let rows = self
            .client
            .query(
                r#"
            UPDATE vendor
            SET name = $1
            WHERE id = $2
            RETURNING id, name, created_at
            "#,
                &[&request.name, &id],
            )
            .await?;

        if let Some(row) = rows.first() {
            Ok(map_row_to_vendor(row))
        } else {
            Err(AppError::NotFound("Vendor not found".to_string()))
        }
    }
}

fn map_row_to_vendor(row: &Row) -> Vendor {
    Vendor {
        id: row.get("id"),
        name: row.get("name"),
        created_at: row.get("created_at"),
    }
}
