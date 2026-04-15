use chrono::{DateTime, NaiveDate, Utc};
use uuid::Uuid;

use crate::crypto::Dek;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::subscriptions::{
    BillingCycle, CreateSubscriptionRequest, EncryptedSubscriptionResponse, SubscriptionStatus, UpdateSubscriptionRequest, to_response,
};
use crate::error::app_error::AppError;

#[derive(sqlx::FromRow)]
struct SubscriptionRow {
    id: Uuid,
    category_id: Uuid,
    vendor_id: Option<Uuid>,
    billing_cycle: BillingCycle,
    billing_day: i16,
    next_charge_date: NaiveDate,
    status: SubscriptionStatus,
    cancelled_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    name_enc: Vec<u8>,
    billing_amount_enc: Vec<u8>,
}

impl From<SubscriptionRow> for EncryptedSubscriptionResponse {
    fn from(row: SubscriptionRow) -> Self {
        to_response(
            row.id,
            row.category_id,
            row.vendor_id,
            row.billing_cycle,
            row.billing_day,
            row.next_charge_date,
            row.status,
            row.cancelled_at,
            row.created_at,
            row.updated_at,
            &row.name_enc,
            &row.billing_amount_enc,
        )
    }
}

const COLS: &str = "id, category_id, vendor_id, billing_cycle::text as billing_cycle, billing_day, next_charge_date, status::text as status, cancelled_at, created_at, updated_at, name_enc, billing_amount_enc";

impl PostgresRepository {
    pub async fn list_subscriptions(&self, user_id: &Uuid) -> Result<Vec<EncryptedSubscriptionResponse>, AppError> {
        let rows: Vec<SubscriptionRow> = sqlx::query_as(&format!("SELECT {COLS} FROM subscription WHERE user_id = $1 ORDER BY id"))
            .bind(user_id)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn create_subscription(&self, req: &CreateSubscriptionRequest, user_id: &Uuid, dek: &Dek) -> Result<EncryptedSubscriptionResponse, AppError> {
        let name_enc = dek.encrypt_string(&req.name)?;
        let amount_enc = dek.encrypt_i64(req.billing_amount)?;
        let row: SubscriptionRow = sqlx::query_as(&format!(
            r#"
INSERT INTO subscription (
    id, user_id, category_id, vendor_id, billing_cycle, billing_day,
    next_charge_date, status, created_at, updated_at, name_enc, billing_amount_enc
) VALUES (
    gen_random_uuid(), $1, $2, $3, $4::text::billing_cycle, $5, $6, 'active'::subscription_status, now(), now(), $7, $8
)
RETURNING {COLS}
"#,
        ))
        .bind(user_id)
        .bind(req.category_id)
        .bind(req.vendor_id)
        .bind(billing_cycle_str(req.billing_cycle))
        .bind(req.billing_day)
        .bind(req.next_charge_date.0)
        .bind(&name_enc)
        .bind(&amount_enc)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.into())
    }

    pub async fn update_subscription(
        &self,
        id: &Uuid,
        req: &UpdateSubscriptionRequest,
        user_id: &Uuid,
        dek: &Dek,
    ) -> Result<EncryptedSubscriptionResponse, AppError> {
        let name_enc = dek.encrypt_string(&req.name)?;
        let amount_enc = dek.encrypt_i64(req.billing_amount)?;
        let row: Option<SubscriptionRow> = sqlx::query_as(&format!(
            r#"
UPDATE subscription
SET category_id = $1,
    vendor_id = $2,
    billing_cycle = $3::text::billing_cycle,
    billing_day = $4,
    next_charge_date = $5,
    name_enc = $6,
    billing_amount_enc = $7,
    updated_at = now()
WHERE id = $8 AND user_id = $9
RETURNING {COLS}
"#,
        ))
        .bind(req.category_id)
        .bind(req.vendor_id)
        .bind(billing_cycle_str(req.billing_cycle))
        .bind(req.billing_day)
        .bind(req.next_charge_date.0)
        .bind(&name_enc)
        .bind(&amount_enc)
        .bind(id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;
        row.map(Into::into).ok_or_else(|| AppError::NotFound("Subscription not found".to_string()))
    }

    pub async fn delete_subscription(&self, id: &Uuid, user_id: &Uuid) -> Result<(), AppError> {
        let result = sqlx::query("DELETE FROM subscription WHERE id = $1 AND user_id = $2")
            .bind(id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("Subscription not found".to_string()));
        }
        Ok(())
    }

    pub async fn cancel_subscription(
        &self,
        id: &Uuid,
        user_id: &Uuid,
        cancellation_date: Option<&NaiveDate>,
    ) -> Result<EncryptedSubscriptionResponse, AppError> {
        let row: Option<SubscriptionRow> = sqlx::query_as(&format!(
            r#"
UPDATE subscription
SET status = 'cancelled'::subscription_status,
    cancelled_at = COALESCE($1::date::timestamptz, now()),
    updated_at = now()
WHERE id = $2 AND user_id = $3
RETURNING {COLS}
"#,
        ))
        .bind(cancellation_date)
        .bind(id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;
        row.map(Into::into).ok_or_else(|| AppError::NotFound("Subscription not found".to_string()))
    }
}

fn billing_cycle_str(c: BillingCycle) -> &'static str {
    match c {
        BillingCycle::Quarterly => "quarterly",
        BillingCycle::Monthly => "monthly",
        BillingCycle::Yearly => "yearly",
    }
}
