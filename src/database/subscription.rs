use chrono::{DateTime, NaiveDate, Utc};
use uuid::Uuid;

use crate::database::postgres_repository::PostgresRepository;
use crate::dto::common::Date;
use crate::dto::subscriptions::{
    BillingCycle, BillingEventResponse, CreateSubscriptionRequest, SubscriptionDetailResponse, SubscriptionResponse, SubscriptionStatus, UpcomingChargeItem,
    UpcomingChargesResponse, UpdateSubscriptionRequest,
};
use crate::error::app_error::AppError;

// ─── Raw rows ────────────────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct SubscriptionRow {
    id: Uuid,
    name: String,
    category_id: Uuid,
    vendor_id: Option<Uuid>,
    billing_amount: i64,
    billing_cycle: String,
    billing_day: i16,
    next_charge_date: NaiveDate,
    status: String,
    cancelled_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct BillingEventRow {
    id: Uuid,
    subscription_id: Uuid,
    transaction_id: Option<Uuid>,
    amount: i64,
    date: NaiveDate,
    detected: bool,
}

#[derive(sqlx::FromRow)]
struct UpcomingRow {
    subscription_id: Uuid,
    name: String,
    billing_amount: i64,
    billing_cycle: String,
    next_charge_date: NaiveDate,
    vendor_id: Option<Uuid>,
    vendor_name: Option<String>,
}

// ─── Conversion helpers ───────────────────────────────────────────────────────

fn billing_cycle_from_db(s: &str) -> BillingCycle {
    match s {
        "quarterly" => BillingCycle::Quarterly,
        "yearly" => BillingCycle::Yearly,
        _ => BillingCycle::Monthly,
    }
}

fn billing_cycle_to_db(c: BillingCycle) -> &'static str {
    match c {
        BillingCycle::Quarterly => "quarterly",
        BillingCycle::Monthly => "monthly",
        BillingCycle::Yearly => "yearly",
    }
}

fn status_from_db(s: &str) -> SubscriptionStatus {
    match s {
        "cancelled" => SubscriptionStatus::Cancelled,
        "paused" => SubscriptionStatus::Paused,
        _ => SubscriptionStatus::Active,
    }
}

fn row_to_response(row: SubscriptionRow) -> SubscriptionResponse {
    SubscriptionResponse {
        id: row.id,
        name: row.name,
        category_id: row.category_id,
        vendor_id: row.vendor_id,
        billing_amount: row.billing_amount,
        billing_cycle: billing_cycle_from_db(&row.billing_cycle),
        billing_day: row.billing_day,
        next_charge_date: Date(row.next_charge_date),
        status: status_from_db(&row.status),
        cancelled_at: row.cancelled_at,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

// ─── Repository methods ───────────────────────────────────────────────────────

impl PostgresRepository {
    pub async fn list_subscriptions(&self, user_id: &Uuid, status_filter: Option<SubscriptionStatus>) -> Result<Vec<SubscriptionResponse>, AppError> {
        let status_str: Option<&str> = status_filter.map(|s| match s {
            SubscriptionStatus::Active => "active",
            SubscriptionStatus::Cancelled => "cancelled",
            SubscriptionStatus::Paused => "paused",
        });

        let rows = sqlx::query_as::<_, SubscriptionRow>(
            r#"
SELECT id, name, category_id, vendor_id, billing_amount,
       billing_cycle::text, billing_day, next_charge_date,
       status::text, cancelled_at, created_at, updated_at
FROM subscription
WHERE user_id = $1
  AND ($2::text IS NULL OR status::text = $2)
ORDER BY created_at DESC
            "#,
        )
        .bind(user_id)
        .bind(status_str)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(row_to_response).collect())
    }

    pub async fn get_subscription(&self, id: &Uuid, user_id: &Uuid) -> Result<Option<SubscriptionResponse>, AppError> {
        let row = sqlx::query_as::<_, SubscriptionRow>(
            r#"
SELECT id, name, category_id, vendor_id, billing_amount,
       billing_cycle::text, billing_day, next_charge_date,
       status::text, cancelled_at, created_at, updated_at
FROM subscription
WHERE id = $1 AND user_id = $2
            "#,
        )
        .bind(id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(row_to_response))
    }

    pub async fn get_subscription_detail(&self, id: &Uuid, user_id: &Uuid) -> Result<Option<SubscriptionDetailResponse>, AppError> {
        let sub = match self.get_subscription(id, user_id).await? {
            Some(s) => s,
            None => return Ok(None),
        };

        let event_rows = sqlx::query_as::<_, BillingEventRow>(
            r#"
SELECT id, subscription_id, transaction_id, amount, date, detected
FROM subscription_billing_event
WHERE subscription_id = $1
ORDER BY date DESC
            "#,
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await?;

        let cancelled_at = sub.cancelled_at;
        let billing_history = event_rows
            .into_iter()
            .map(|e| {
                let post_cancellation = cancelled_at.map(|ca| e.date > ca.date_naive()).unwrap_or(false);
                BillingEventResponse {
                    id: e.id,
                    subscription_id: e.subscription_id,
                    transaction_id: e.transaction_id,
                    amount: e.amount,
                    date: Date(e.date),
                    detected: e.detected,
                    post_cancellation,
                }
            })
            .collect();

        Ok(Some(SubscriptionDetailResponse {
            subscription: sub,
            billing_history,
        }))
    }

    pub async fn create_subscription(&self, req: &CreateSubscriptionRequest, user_id: &Uuid) -> Result<SubscriptionResponse, AppError> {
        let row = sqlx::query_as::<_, SubscriptionRow>(
            r#"
INSERT INTO subscription
    (user_id, name, category_id, vendor_id, billing_amount, billing_cycle, billing_day, next_charge_date)
VALUES
    ($1, $2, $3, $4, $5, $6::subscription_billing_cycle, $7, $8)
RETURNING id, name, category_id, vendor_id, billing_amount,
          billing_cycle::text, billing_day, next_charge_date,
          status::text, cancelled_at, created_at, updated_at
            "#,
        )
        .bind(user_id)
        .bind(&req.name)
        .bind(req.category_id)
        .bind(req.vendor_id)
        .bind(req.billing_amount)
        .bind(billing_cycle_to_db(req.billing_cycle))
        .bind(req.billing_day)
        .bind(req.next_charge_date.0)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match &e {
            sqlx::Error::Database(db_err) if db_err.is_foreign_key_violation() => {
                AppError::BadRequest("Referenced category or vendor does not exist".to_string())
            }
            _ => e.into(),
        })?;

        Ok(row_to_response(row))
    }

    pub async fn update_subscription(&self, id: &Uuid, req: &UpdateSubscriptionRequest, user_id: &Uuid) -> Result<Option<SubscriptionResponse>, AppError> {
        let row = sqlx::query_as::<_, SubscriptionRow>(
            r#"
UPDATE subscription
SET name = $3,
    category_id = $4,
    vendor_id = $5,
    billing_amount = $6,
    billing_cycle = $7::subscription_billing_cycle,
    billing_day = $8,
    next_charge_date = $9,
    updated_at = NOW()
WHERE id = $1 AND user_id = $2
RETURNING id, name, category_id, vendor_id, billing_amount,
          billing_cycle::text, billing_day, next_charge_date,
          status::text, cancelled_at, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(user_id)
        .bind(&req.name)
        .bind(req.category_id)
        .bind(req.vendor_id)
        .bind(req.billing_amount)
        .bind(billing_cycle_to_db(req.billing_cycle))
        .bind(req.billing_day)
        .bind(req.next_charge_date.0)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(row_to_response))
    }

    pub async fn delete_subscription(&self, id: &Uuid, user_id: &Uuid) -> Result<bool, AppError> {
        let result = sqlx::query("DELETE FROM subscription WHERE id = $1 AND user_id = $2")
            .bind(id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn cancel_subscription(
        &self,
        id: &Uuid,
        user_id: &Uuid,
        cancellation_date: Option<&chrono::NaiveDate>,
    ) -> Result<Option<SubscriptionResponse>, AppError> {
        let row = sqlx::query_as::<_, SubscriptionRow>(
            r#"
UPDATE subscription
SET status = 'cancelled',
    cancelled_at = COALESCE($3::date, NOW()::date)::timestamp AT TIME ZONE 'UTC',
    updated_at = NOW()
WHERE id = $1 AND user_id = $2 AND status != 'cancelled'
RETURNING id, name, category_id, vendor_id, billing_amount,
          billing_cycle::text, billing_day, next_charge_date,
          status::text, cancelled_at, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(user_id)
        .bind(cancellation_date)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(row_to_response))
    }

    pub async fn get_upcoming_charges(&self, user_id: &Uuid, limit: i64) -> Result<UpcomingChargesResponse, AppError> {
        let rows = sqlx::query_as::<_, UpcomingRow>(
            r#"
SELECT
    s.id AS subscription_id,
    s.name,
    s.billing_amount,
    s.billing_cycle::text AS billing_cycle,
    s.next_charge_date,
    s.vendor_id,
    v.name AS vendor_name
FROM subscription s
LEFT JOIN vendor v ON v.id = s.vendor_id
WHERE s.user_id = $1
  AND s.status = 'active'
ORDER BY s.next_charge_date ASC
LIMIT $2
            "#,
        )
        .bind(user_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| UpcomingChargeItem {
                subscription_id: r.subscription_id,
                name: r.name,
                billing_amount: r.billing_amount,
                billing_cycle: billing_cycle_from_db(&r.billing_cycle),
                next_charge_date: Date(r.next_charge_date),
                vendor_id: r.vendor_id,
                vendor_name: r.vendor_name,
            })
            .collect())
    }
}
