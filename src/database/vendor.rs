use crate::database::postgres_repository::{PostgresRepository, is_unique_violation};
use crate::error::app_error::AppError;
use crate::models::budget_period::BudgetPeriod;
use crate::models::pagination::CursorParams;
use crate::models::vendor::{Vendor, VendorPeriodStats, VendorRequest, VendorStats, VendorWithPeriodStats, VendorWithStats};
use chrono::NaiveDate;
use rocket::FromFormField;
use schemars::JsonSchema;
use uuid::Uuid;

// ─── Helper DB rows (vendor analytics) ───────────────────────────────────────

#[derive(sqlx::FromRow)]
pub struct PeriodDateRow {
    #[allow(dead_code)]
    pub id: Uuid,
    #[allow(dead_code)]
    pub name: String,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
}

#[derive(sqlx::FromRow)]
pub struct VendorPeriodStatsRow {
    pub transaction_count: i64,
    pub period_spend: i64,
}

#[derive(sqlx::FromRow)]
pub struct VendorTrendRow {
    pub period_id: Uuid,
    pub period_name: String,
    pub total_spend: i64,
}

#[derive(sqlx::FromRow)]
pub struct VendorCategoryRow {
    pub category_id: Uuid,
    pub category_name: String,
    pub total_spend: i64,
}

#[derive(sqlx::FromRow)]
pub struct VendorRecentTxRow {
    pub id: Uuid,
    pub date: NaiveDate,
    pub amount: i64,
    pub description: String,
    pub category_id: Option<Uuid>,
    pub category_name: Option<String>,
}

pub struct VendorDetailDb {
    pub vendor: Vendor,
    pub period_spend: i64,
    pub transaction_count: i64,
    pub total_vendor_spend: i64,
    pub trend: Vec<VendorTrendRow>,
    pub top_categories: Vec<VendorCategoryRow>,
    pub recent_txns: Vec<VendorRecentTxRow>,
}

pub struct VendorStatsDb {
    pub total_vendors: i64,
    pub total_spend: i64,
    pub avg_spend_per_vendor: i64,
}

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
            INSERT INTO vendor (user_id, name, description)
            VALUES ($1, $2, $3)
            RETURNING id, name, description, archived
            "#,
        )
        .bind(user_id)
        .bind(&request.name)
        .bind(&request.description)
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
            SELECT id, name, description, archived
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

    pub async fn list_vendors(
        &self,
        params: &CursorParams,
        user_id: &Uuid,
        period: &BudgetPeriod,
        include_archived: bool,
    ) -> Result<Vec<VendorWithPeriodStats>, AppError> {
        #[derive(sqlx::FromRow)]
        struct VendorWithPeriodStatsRow {
            id: Uuid,
            name: String,
            description: Option<String>,
            archived: bool,
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
       v.name,
       v.description,
       v.archived,
       COUNT(t.id) FILTER (
            WHERE t.occurred_at >= sp.start_date
              AND t.occurred_at <= sp.end_date
       )::bigint AS transaction_count,
       MAX(t.occurred_at) AS last_used_at
FROM vendor v
CROSS JOIN selected_period sp
LEFT JOIN transaction t ON v.id = t.vendor_id AND t.user_id = $1
WHERE v.user_id = $1
  AND (v.archived = FALSE OR $5)
  AND (v.created_at, v.id) < (SELECT created_at, id FROM vendor WHERE id = $4)
GROUP BY v.id, v.user_id, v.name, v.description, v.archived, v.created_at
ORDER BY v.created_at DESC, v.id DESC
LIMIT $6
                "#,
            )
            .bind(user_id)
            .bind(period.start_date)
            .bind(period.end_date)
            .bind(cursor)
            .bind(include_archived)
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
       v.name,
       v.description,
       v.archived,
       COUNT(t.id) FILTER (
            WHERE t.occurred_at >= sp.start_date
              AND t.occurred_at <= sp.end_date
       )::bigint AS transaction_count,
       MAX(t.occurred_at) AS last_used_at
FROM vendor v
CROSS JOIN selected_period sp
LEFT JOIN transaction t ON v.id = t.vendor_id AND t.user_id = $1
WHERE v.user_id = $1
  AND (v.archived = FALSE OR $4)
GROUP BY v.id, v.user_id, v.name, v.description, v.archived, v.created_at
ORDER BY v.created_at DESC, v.id DESC
LIMIT $5
                "#,
            )
            .bind(user_id)
            .bind(period.start_date)
            .bind(period.end_date)
            .bind(include_archived)
            .bind(params.fetch_limit())
            .fetch_all(&self.pool)
            .await?
        };

        Ok(rows
            .into_iter()
            .map(|row| VendorWithPeriodStats {
                vendor: Vendor {
                    id: row.id,
                    name: row.name,
                    description: row.description,
                    archived: row.archived,
                },
                stats: VendorPeriodStats {
                    transaction_count: row.transaction_count,
                    last_used_at: row.last_used_at,
                },
            })
            .collect())
    }

    pub async fn list_vendors_with_status(&self, order_by: VendorOrderBy, archived: bool, user_id: &Uuid) -> Result<Vec<VendorWithStats>, AppError> {
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
            description: Option<String>,
            archived: bool,
            transaction_count: i64,
            last_used_at: Option<NaiveDate>,
        }

        let query = format!(
            r#"
            SELECT v.id,
                   v.name,
                   v.description,
                   v.archived,
                   COUNT(t.id) AS transaction_count,
                   MAX(t.occurred_at) AS last_used_at
            FROM vendor v
            LEFT JOIN transaction t ON v.id = t.vendor_id
            WHERE v.user_id = $1
              AND v.archived = $2
            GROUP BY v.id, v.user_id, v.name, v.description, v.archived, v.created_at
            ORDER BY {} ASC NULLS LAST
            "#,
            order_by_clause
        );

        let rows = sqlx::query_as::<_, VendorWithStatsRow>(&query)
            .bind(user_id)
            .bind(archived)
            .fetch_all(&self.pool)
            .await?;

        Ok(rows
            .into_iter()
            .map(|r| VendorWithStats {
                vendor: Vendor {
                    id: r.id,
                    name: r.name,
                    description: r.description,
                    archived: r.archived,
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
            SET name = $1, description = $2
            WHERE id = $3 AND user_id = $4
            RETURNING id, name, description, archived
            "#,
        )
        .bind(&request.name)
        .bind(&request.description)
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

    /// Returns all **non-archived** vendors for the user, ordered by name.
    ///
    /// This intentionally excludes archived vendors because the result is used
    /// to populate transaction-creation dropdowns. Archived vendors should not
    /// be assignable to new transactions. Use [`list_vendors_with_status`] with
    /// `archived = true` if you need to enumerate archived vendors.
    pub async fn list_all_vendors(&self, user_id: &Uuid) -> Result<Vec<Vendor>, AppError> {
        let vendors = sqlx::query_as::<_, Vendor>(
            r#"
            SELECT id, name, description, archived
            FROM vendor
            WHERE user_id = $1 AND archived = FALSE
            ORDER BY name ASC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(vendors)
    }

    pub async fn archive_vendor(&self, id: &Uuid, user_id: &Uuid) -> Result<Vendor, AppError> {
        let vendor = sqlx::query_as::<_, Vendor>(
            r#"
            UPDATE vendor
            SET archived = TRUE
            WHERE id = $1 AND user_id = $2
            RETURNING id, name, description, archived
            "#,
        )
        .bind(id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        vendor.ok_or_else(|| AppError::NotFound("Vendor not found".to_string()))
    }

    /// Lists vendors with all-time transaction count and total spend for V2 paginated list.
    /// Returns `(rows, total_count)`. Fetches `limit + 1` rows so the caller can detect `has_more`.
    pub async fn list_vendors_v2(&self, cursor: Option<Uuid>, limit: i64, user_id: &Uuid) -> Result<(Vec<(Vendor, i64, i64)>, i64), AppError> {
        let total_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM vendor WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(&self.pool)
            .await?;

        #[derive(sqlx::FromRow)]
        struct Row {
            id: Uuid,
            name: String,
            description: Option<String>,
            archived: bool,
            transaction_count: i64,
            total_spend: i64,
        }

        let fetch_limit = limit + 1;

        let rows = if let Some(cursor_id) = cursor {
            sqlx::query_as::<_, Row>(
                r#"
SELECT v.id,
       v.name,
       v.description,
       v.archived,
       COUNT(t.id)::bigint AS transaction_count,
       COALESCE(SUM(t.amount), 0)::bigint AS total_spend
FROM vendor v
LEFT JOIN transaction t ON v.id = t.vendor_id AND t.user_id = $1
WHERE v.user_id = $1
  AND (v.created_at, v.id) < (SELECT created_at, id FROM vendor WHERE id = $2 AND user_id = $1)
GROUP BY v.id, v.name, v.description, v.archived, v.created_at
ORDER BY v.created_at DESC, v.id DESC
LIMIT $3
                "#,
            )
            .bind(user_id)
            .bind(cursor_id)
            .bind(fetch_limit)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, Row>(
                r#"
SELECT v.id,
       v.name,
       v.description,
       v.archived,
       COUNT(t.id)::bigint AS transaction_count,
       COALESCE(SUM(t.amount), 0)::bigint AS total_spend
FROM vendor v
LEFT JOIN transaction t ON v.id = t.vendor_id AND t.user_id = $1
WHERE v.user_id = $1
GROUP BY v.id, v.name, v.description, v.archived, v.created_at
ORDER BY v.created_at DESC, v.id DESC
LIMIT $2
                "#,
            )
            .bind(user_id)
            .bind(fetch_limit)
            .fetch_all(&self.pool)
            .await?
        };

        Ok((
            rows.into_iter()
                .map(|r| {
                    (
                        Vendor {
                            id: r.id,
                            name: r.name,
                            description: r.description,
                            archived: r.archived,
                        },
                        r.transaction_count,
                        r.total_spend,
                    )
                })
                .collect(),
            total_count,
        ))
    }

    pub async fn get_vendor_detail_v2(&self, vendor_id: &Uuid, user_id: &Uuid, period_id: &Uuid) -> Result<Option<VendorDetailDb>, AppError> {
        // 1. Verify vendor exists
        let vendor = match self.get_vendor_by_id(vendor_id, user_id).await? {
            Some(v) => v,
            None => return Ok(None),
        };

        // 2. Period date range
        let period = sqlx::query_as::<_, PeriodDateRow>("SELECT id, name, start_date, end_date FROM budget_period WHERE id = $1 AND user_id = $2")
            .bind(period_id)
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await?;

        let period = match period {
            Some(p) => p,
            None => return Err(AppError::NotFound("Period not found".to_string())),
        };

        // 3. Period spend + tx count
        let stats = sqlx::query_as::<_, VendorPeriodStatsRow>(
            r#"
SELECT
    COUNT(t.id)::bigint AS transaction_count,
    COALESCE(SUM(t.amount), 0)::bigint AS period_spend
FROM transaction t
WHERE t.vendor_id = $1
  AND t.user_id = $2
  AND t.occurred_at >= $3
  AND t.occurred_at <= $4
            "#,
        )
        .bind(vendor_id)
        .bind(user_id)
        .bind(period.start_date)
        .bind(period.end_date)
        .fetch_one(&self.pool)
        .await?;

        // 4. Trend: last 6 periods ordered by start_date
        let trend = sqlx::query_as::<_, VendorTrendRow>(
            r#"
SELECT
    bp.id AS period_id,
    bp.name AS period_name,
    COALESCE(SUM(t.amount), 0)::bigint AS total_spend
FROM budget_period bp
LEFT JOIN transaction t
    ON t.vendor_id = $1
    AND t.user_id = $2
    AND t.occurred_at >= bp.start_date
    AND t.occurred_at <= bp.end_date
WHERE bp.user_id = $2
GROUP BY bp.id, bp.name, bp.start_date
ORDER BY bp.start_date DESC
LIMIT 6
            "#,
        )
        .bind(vendor_id)
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        // 5. Top categories (all-time, top 5)
        let total_vendor_spend: i64 = sqlx::query_scalar("SELECT COALESCE(SUM(amount), 0)::bigint FROM transaction WHERE vendor_id = $1 AND user_id = $2")
            .bind(vendor_id)
            .bind(user_id)
            .fetch_one(&self.pool)
            .await?;

        let top_categories = sqlx::query_as::<_, VendorCategoryRow>(
            r#"
SELECT
    c.id AS category_id,
    c.name AS category_name,
    COALESCE(SUM(t.amount), 0)::bigint AS total_spend
FROM transaction t
JOIN category c ON c.id = t.category_id
WHERE t.vendor_id = $1
  AND t.user_id = $2
GROUP BY c.id, c.name
ORDER BY total_spend DESC
LIMIT 5
            "#,
        )
        .bind(vendor_id)
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        // 6. Recent transactions (last 10)
        let recent_txns = sqlx::query_as::<_, VendorRecentTxRow>(
            r#"
SELECT
    t.id,
    t.occurred_at AS date,
    t.amount,
    t.description,
    t.category_id,
    c.name AS category_name
FROM transaction t
LEFT JOIN category c ON c.id = t.category_id
WHERE t.vendor_id = $1
  AND t.user_id = $2
ORDER BY t.occurred_at DESC, t.id DESC
LIMIT 10
            "#,
        )
        .bind(vendor_id)
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(Some(VendorDetailDb {
            vendor,
            period_spend: stats.period_spend,
            transaction_count: stats.transaction_count,
            total_vendor_spend,
            trend,
            top_categories,
            recent_txns,
        }))
    }

    pub async fn merge_vendor(&self, source_id: &Uuid, target_id: &Uuid, user_id: &Uuid) -> Result<bool, AppError> {
        // Verify both vendors belong to this user
        let source = self.get_vendor_by_id(source_id, user_id).await?;
        if source.is_none() {
            return Ok(false);
        }
        let target = self.get_vendor_by_id(target_id, user_id).await?;
        if target.is_none() {
            return Err(AppError::NotFound("Target vendor not found".to_string()));
        }

        // Perform reassignment and deletion atomically
        let mut tx = self.pool.begin().await?;

        // Reassign all transactions from source to target
        sqlx::query("UPDATE transaction SET vendor_id = $1 WHERE vendor_id = $2 AND user_id = $3")
            .bind(target_id)
            .bind(source_id)
            .bind(user_id)
            .execute(&mut *tx)
            .await?;

        // Delete the source vendor
        sqlx::query("DELETE FROM vendor WHERE id = $1 AND user_id = $2")
            .bind(source_id)
            .bind(user_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;

        Ok(true)
    }

    pub async fn get_vendor_stats_v2(&self, user_id: &Uuid, period_id: &Uuid) -> Result<VendorStatsDb, AppError> {
        let period = sqlx::query_as::<_, PeriodDateRow>("SELECT id, name, start_date, end_date FROM budget_period WHERE id = $1 AND user_id = $2")
            .bind(period_id)
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await?;

        let period = period.ok_or_else(|| AppError::NotFound("Period not found".to_string()))?;

        let total_vendors: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM vendor WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(&self.pool)
            .await?;

        let total_spend: i64 = sqlx::query_scalar(
            r#"
SELECT COALESCE(SUM(t.amount), 0)::bigint
FROM transaction t
JOIN vendor v ON v.id = t.vendor_id
WHERE t.user_id = $1
  AND t.occurred_at >= $2
  AND t.occurred_at <= $3
            "#,
        )
        .bind(user_id)
        .bind(period.start_date)
        .bind(period.end_date)
        .fetch_one(&self.pool)
        .await?;

        let vendors_with_spend: i64 = sqlx::query_scalar(
            r#"
SELECT COUNT(DISTINCT t.vendor_id)
FROM transaction t
WHERE t.user_id = $1
  AND t.vendor_id IS NOT NULL
  AND t.occurred_at >= $2
  AND t.occurred_at <= $3
            "#,
        )
        .bind(user_id)
        .bind(period.start_date)
        .bind(period.end_date)
        .fetch_one(&self.pool)
        .await?;

        let avg_spend_per_vendor = if vendors_with_spend > 0 { total_spend / vendors_with_spend } else { 0 };

        Ok(VendorStatsDb {
            total_vendors,
            total_spend,
            avg_spend_per_vendor,
        })
    }

    pub async fn restore_vendor(&self, id: &Uuid, user_id: &Uuid) -> Result<Vendor, AppError> {
        let vendor = sqlx::query_as::<_, Vendor>(
            r#"
            UPDATE vendor
            SET archived = FALSE
            WHERE id = $1 AND user_id = $2
            RETURNING id, name, description, archived
            "#,
        )
        .bind(id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        vendor.ok_or_else(|| AppError::NotFound("Vendor not found".to_string()))
    }
}
