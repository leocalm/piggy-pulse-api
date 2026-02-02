use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::models::pagination::PaginationParams;
use crate::models::vendor::{Vendor, VendorRequest, VendorStats, VendorWithStats};
use chrono::{DateTime, NaiveDate, Utc};
use rocket::FromFormField;
use uuid::Uuid;

#[derive(FromFormField, Debug, Clone, Copy)]
pub enum VendorOrderBy {
    #[field(value = "name")]
    Name,
    #[field(value = "most_used")]
    MostUsed,
    #[field(value = "more_recent")]
    MoreRecent,
}

#[async_trait::async_trait]
pub trait VendorRepository {
    async fn create_vendor(&self, request: &VendorRequest) -> Result<Vendor, AppError>;
    async fn get_vendor_by_id(&self, id: &Uuid) -> Result<Option<Vendor>, AppError>;
    async fn list_vendors(&self, pagination: Option<&PaginationParams>) -> Result<(Vec<Vendor>, i64), AppError>;
    async fn list_vendors_with_status(&self, order_by: VendorOrderBy) -> Result<Vec<VendorWithStats>, AppError>;
    async fn delete_vendor(&self, id: &Uuid) -> Result<(), AppError>;
    async fn update_vendor(&self, id: &Uuid, request: &VendorRequest) -> Result<Vendor, AppError>;
}

#[async_trait::async_trait]
impl VendorRepository for PostgresRepository {
    async fn create_vendor(&self, request: &VendorRequest) -> Result<Vendor, AppError> {
        let vendor = sqlx::query_as!(
            Vendor,
            r#"
            INSERT INTO vendor (name)
            VALUES ($1)
            RETURNING id, name, created_at
            "#,
            &request.name
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(vendor)
    }

    async fn get_vendor_by_id(&self, id: &Uuid) -> Result<Option<Vendor>, AppError> {
        let vendor = sqlx::query_as!(
            Vendor,
            r#"
            SELECT id, name, created_at
            FROM vendor
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(vendor)
    }

    async fn list_vendors(&self, pagination: Option<&PaginationParams>) -> Result<(Vec<Vendor>, i64), AppError> {
        // Get total count
        #[derive(sqlx::FromRow)]
        struct CountRow {
            total: i64,
        }

        let count_row = sqlx::query_as::<_, CountRow>("SELECT COUNT(*) as total FROM vendor")
            .fetch_one(&self.pool)
            .await?;
        let total = count_row.total;

        // Build query with optional pagination
        let mut query = String::from(
            r#"
            SELECT id, name, created_at
            FROM vendor
            ORDER BY created_at DESC
            "#,
        );

        // Add pagination if requested
        if let Some(params) = pagination
            && let (Some(limit), Some(offset)) = (params.effective_limit(), params.offset())
        {
            query.push_str(&format!(" LIMIT {} OFFSET {}", limit, offset));
        }

        let vendors = sqlx::query_as::<_, Vendor>(&query).fetch_all(&self.pool).await?;

        Ok((vendors, total))
    }

    async fn list_vendors_with_status(&self, order_by: VendorOrderBy) -> Result<Vec<VendorWithStats>, AppError> {
        // Safe from SQL injection: order_by_clause is derived from a controlled enum
        let order_by_clause = match order_by {
            VendorOrderBy::Name => "v.name",
            VendorOrderBy::MostUsed => "transaction_count",
            VendorOrderBy::MoreRecent => "last_used_at",
        };

        #[derive(sqlx::FromRow)]
        struct VendorWithStatsRow {
            id: Uuid,
            name: String,
            created_at: DateTime<Utc>,
            transaction_count: i64,
            last_used_at: Option<NaiveDate>,
        }

        let query = format!(
            r#"
            SELECT v.id,
                   v.name,
                   v.created_at,
                   COUNT(t.id) AS transaction_count,
                   MAX(t.occurred_at) AS last_used_at
            FROM vendor v
            LEFT JOIN transaction t ON v.id = t.vendor_id
            GROUP BY v.id, v.name, v.created_at
            ORDER BY {} ASC NULLS LAST
            "#,
            order_by_clause
        );

        let rows = sqlx::query_as::<_, VendorWithStatsRow>(&query).fetch_all(&self.pool).await?;

        Ok(rows
            .into_iter()
            .map(|r| VendorWithStats {
                vendor: Vendor {
                    id: r.id,
                    name: r.name,
                    created_at: r.created_at,
                },
                stats: VendorStats {
                    transaction_count: r.transaction_count,
                    last_used_at: r.last_used_at,
                },
            })
            .collect())
    }

    async fn delete_vendor(&self, id: &Uuid) -> Result<(), AppError> {
        sqlx::query("DELETE FROM vendor WHERE id = $1").bind(id).execute(&self.pool).await?;
        Ok(())
    }

    async fn update_vendor(&self, id: &Uuid, request: &VendorRequest) -> Result<Vendor, AppError> {
        let vendor = sqlx::query_as!(
            Vendor,
            r#"
            UPDATE vendor
            SET name = $1
            WHERE id = $2
            RETURNING id, name, created_at
            "#,
            &request.name,
            id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(vendor)
    }
}
