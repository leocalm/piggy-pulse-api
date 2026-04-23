use base64::Engine;
use base64::engine::general_purpose::STANDARD as B64;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::dto::common::Date;

fn b64(bytes: &[u8]) -> String {
    B64.encode(bytes)
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "text", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum BillingCycle {
    Quarterly,
    Monthly,
    Yearly,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "text", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum SubscriptionStatus {
    Active,
    Cancelled,
    Paused,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct EncryptedSubscriptionResponse {
    pub id: Uuid,
    pub category_id: Uuid,
    pub vendor_id: Option<Uuid>,
    pub billing_cycle: BillingCycle,
    pub billing_day: i16,
    pub next_charge_date: Date,
    pub status: SubscriptionStatus,
    pub cancelled_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub name_enc: String,
    pub billing_amount_enc: String,
}

pub type SubscriptionListResponse = Vec<EncryptedSubscriptionResponse>;

#[derive(Deserialize, Debug, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateSubscriptionRequest {
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    pub category_id: Uuid,
    pub vendor_id: Option<Uuid>,
    #[validate(range(min = 1))]
    pub billing_amount: i64,
    pub billing_cycle: BillingCycle,
    #[validate(range(min = 1, max = 31))]
    pub billing_day: i16,
    pub next_charge_date: Date,
}

pub type UpdateSubscriptionRequest = CreateSubscriptionRequest;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CancelSubscriptionRequest {
    pub cancellation_date: Option<Date>,
}

#[allow(clippy::too_many_arguments)]
pub fn to_response(
    id: Uuid,
    category_id: Uuid,
    vendor_id: Option<Uuid>,
    billing_cycle: BillingCycle,
    billing_day: i16,
    next_charge_date: chrono::NaiveDate,
    status: SubscriptionStatus,
    cancelled_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    name_enc: &[u8],
    billing_amount_enc: &[u8],
) -> EncryptedSubscriptionResponse {
    EncryptedSubscriptionResponse {
        id,
        category_id,
        vendor_id,
        billing_cycle,
        billing_day,
        next_charge_date: Date(next_charge_date),
        status,
        cancelled_at,
        created_at,
        updated_at,
        name_enc: b64(name_enc),
        billing_amount_enc: b64(billing_amount_enc),
    }
}
