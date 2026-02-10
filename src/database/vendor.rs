use crate::database::postgres_repository::{PostgresRepository, is_unique_violation};
use crate::error::app_error::AppError;
use crate::models::budget_period::BudgetPeriod;
use crate::models::pagination::CursorParams;
use crate::models::vendor::{Vendor, VendorPeriodStats, VendorRequest, VendorStats, VendorWithPeriodStats, VendorWithStats};
use chrono::{DateTime, NaiveDate, Utc};
use rocket::FromFormField;
use schemars::JsonSchema;
use uuid::Uuid;

#[derive(FromFormField, Debug, Clone, Copy, JsonSchema)]
pub enum VendorOrderBy {
    #[field(value = "name")]
    Name,
    #[field(value = "most_used")]
    MostUsed,
    #[field(value = "more_recent")]
    MoreRecent,
}

impl PostgresRepository {
    pub async fn create_vendor(&self, request: &VendorRequest, user_id: &Uuid) -> Result<Vendor, AppError> {
        let name_exists: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS (
                SELECT 1
                FROM vendor
                WHERE user_id = $1 AND name = $2
            )
            "#,
        )
        .bind(user_id)
        .bind(&request.name)
        .fetch_one(&self.pool)
        .await?;

        if name_exists {
            return Err(AppError::BadRequest("Vendor name already exists".to_string()));
        }

        let vendor = sqlx::query_as::<_, Vendor>(
            r#"
            INSERT INTO vendor (user_id, name)
            VALUES ($1, $2)
            RETURNING id, user_id, name, created_at
            "#,
        )
        .bind(user_id)
        .bind(&request.name)
        .fetch_one(&self.pool)
        .await;

        let vendor = match vendor {
            Ok(vendor) => vendor,
            Err(err) if is_unique_violation(&err) => {
                return Err(AppError::BadRequest("Vendor name already exists".to_string()));
            }
            Err(err) => return Err(err.into()),
        };

        Ok(vendor)
    }

    pub async fn get_vendor_by_id(&self, id: &Uuid, user_id: &Uuid) -> Result<Option<Vendor>, AppError> {
        let vendor = sqlx::query_as::<_, Vendor>(
            r#"
            SELECT id, user_id, name, created_at
            FROM vendor
            WHERE id = $1 AND user_id = $2
            "#,
        )
        .bind(id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(vendor)
    }

    pub async fn list_vendors(&self, params: &CursorParams, user_id: &Uuid, period: &BudgetPeriod) -> Result<Vec<VendorWithPeriodStats>, AppError> {
        #[derive(sqlx::FromRow)]
        struct VendorWithPeriodStatsRow {
            id: Uuid,
            user_id: Uuid,
            name: String,
            created_at: DateTime<Utc>,
            transaction_count: i64,
            last_used_at: Option<NaiveDate>,
        }

        let rows = if let Some(cursor) = params.cursor {
            sqlx::query_as::<_, VendorWithPeriodStatsRow>(
                r#"
WITH selected_period AS (
    SELECT $2::date AS start_date, $3::date AS end_date
)
SELECT v.id,
       v.user_id,
       v.name,
       v.created_at,
       COUNT(t.id) FILTER (
            WHERE t.occurred_at >= sp.start_date
              AND t.occurred_at <= sp.end_date
       )::bigint AS transaction_count,
       MAX(t.occurred_at) AS last_used_at
FROM vendor v
CROSS JOIN selected_period sp
LEFT JOIN transaction t ON v.id = t.vendor_id AND t.user_id = $1
WHERE v.user_id = $1
  AND (v.created_at, v.id) < (SELECT created_at, id FROM vendor WHERE id = $4)
GROUP BY v.id, v.user_id, v.name, v.created_at
ORDER BY v.created_at DESC, v.id DESC
LIMIT $5
                "#,
            )
            .bind(user_id)
            .bind(period.start_date)
            .bind(period.end_date)
            .bind(cursor)
            .bind(params.fetch_limit())
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, VendorWithPeriodStatsRow>(
                r#"
WITH selected_period AS (
    SELECT $2::date AS start_date, $3::date AS end_date
)
SELECT v.id,
       v.user_id,
       v.name,
       v.created_at,
       COUNT(t.id) FILTER (
            WHERE t.occurred_at >= sp.start_date
              AND t.occurred_at <= sp.end_date
       )::bigint AS transaction_count,
       MAX(t.occurred_at) AS last_used_at
FROM vendor v
CROSS JOIN selected_period sp
LEFT JOIN transaction t ON v.id = t.vendor_id AND t.user_id = $1
WHERE v.user_id = $1
GROUP BY v.id, v.user_id, v.name, v.created_at
ORDER BY v.created_at DESC, v.id DESC
LIMIT $4
                "#,
            )
            .bind(user_id)
            .bind(period.start_date)
            .bind(period.end_date)
            .bind(params.fetch_limit())
            .fetch_all(&self.pool)
            .await?
        };

        Ok(rows
            .into_iter()
            .map(|row| VendorWithPeriodStats {
                vendor: Vendor {
                    id: row.id,
                    user_id: row.user_id,
                    name: row.name,
                    created_at: row.created_at,
                },
                stats: VendorPeriodStats {
                    transaction_count: row.transaction_count,
                    last_used_at: row.last_used_at,
                },
            })
            .collect())
    }

    pub async fn list_vendors_with_status(&self, order_by: VendorOrderBy, user_id: &Uuid) -> Result<Vec<VendorWithStats>, AppError> {
        // Safe from SQL injection: order_by_clause is derived from a controlled enum
        let order_by_clause = match order_by {
            VendorOrderBy::Name => "v.name",
            VendorOrderBy::MostUsed => "transaction_count",
            VendorOrderBy::MoreRecent => "last_used_at",
        };

        #[derive(sqlx::FromRow)]
        struct VendorWithStatsRow {
            id: Uuid,
            user_id: Uuid,
            name: String,
            created_at: DateTime<Utc>,
            transaction_count: i64,
            last_used_at: Option<NaiveDate>,
        }

        let query = format!(
            r#"
            SELECT v.id,
                   v.user_id,
                   v.name,
                   v.created_at,
                   COUNT(t.id) AS transaction_count,
                   MAX(t.occurred_at) AS last_used_at
            FROM vendor v
            LEFT JOIN transaction t ON v.id = t.vendor_id
            WHERE v.user_id = $1
            GROUP BY v.id, v.user_id, v.name, v.created_at
            ORDER BY {} ASC NULLS LAST
            "#,
            order_by_clause
        );

        let rows = sqlx::query_as::<_, VendorWithStatsRow>(&query).bind(user_id).fetch_all(&self.pool).await?;

        Ok(rows
            .into_iter()
            .map(|r| VendorWithStats {
                vendor: Vendor {
                    id: r.id,
                    user_id: r.user_id,
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

    pub async fn delete_vendor(&self, id: &Uuid, user_id: &Uuid) -> Result<(), AppError> {
        sqlx::query("DELETE FROM vendor WHERE id = $1 AND user_id = $2")
            .bind(id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn update_vendor(&self, id: &Uuid, request: &VendorRequest, user_id: &Uuid) -> Result<Vendor, AppError> {
        let name_exists: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS (
                SELECT 1
                FROM vendor
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
            return Err(AppError::BadRequest("Vendor name already exists".to_string()));
        }

        let vendor = sqlx::query_as::<_, Vendor>(
            r#"
            UPDATE vendor
            SET name = $1
            WHERE id = $2 AND user_id = $3
            RETURNING id, user_id, name, created_at
            "#,
        )
        .bind(&request.name)
        .bind(id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await;

        let vendor = match vendor {
            Ok(vendor) => vendor,
            Err(err) if is_unique_violation(&err) => {
                return Err(AppError::BadRequest("Vendor name already exists".to_string()));
            }
            Err(err) => return Err(err.into()),
        };

        Ok(vendor)
    }

    pub async fn list_all_vendors(&self, user_id: &Uuid) -> Result<Vec<Vendor>, AppError> {
        let vendors = sqlx::query_as::<_, Vendor>(
            r#"
            SELECT id, user_id, name, created_at
            FROM vendor
            WHERE user_id = $1
            ORDER BY name ASC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(vendors)
    }
}
