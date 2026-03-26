use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::dto::common::Date;

// ===== Enums =====

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BillingCycle {
    Quarterly,
    Monthly,
    Yearly,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SubscriptionStatus {
    Active,
    Cancelled,
    Paused,
}

// ===== SubscriptionResponse =====

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionResponse {
    pub id: Uuid,
    pub name: String,
    pub category_id: Uuid,
    pub vendor_id: Option<Uuid>,
    pub billing_amount: i64,
    pub billing_cycle: BillingCycle,
    pub billing_day: i16,
    pub next_charge_date: Date,
    pub status: SubscriptionStatus,
    pub cancelled_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ===== SubscriptionDetailResponse =====

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BillingEventResponse {
    pub id: Uuid,
    pub subscription_id: Uuid,
    pub transaction_id: Option<Uuid>,
    pub amount: i64,
    pub date: Date,
    pub detected: bool,
    pub post_cancellation: bool,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionDetailResponse {
    #[serde(flatten)]
    pub subscription: SubscriptionResponse,
    pub billing_history: Vec<BillingEventResponse>,
}

pub type SubscriptionListResponse = Vec<SubscriptionResponse>;

// ===== UpcomingChargeResponse =====

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UpcomingChargeItem {
    pub subscription_id: Uuid,
    pub name: String,
    pub billing_amount: i64,
    pub billing_cycle: BillingCycle,
    pub next_charge_date: Date,
    pub vendor_id: Option<Uuid>,
    pub vendor_name: Option<String>,
}

pub type UpcomingChargesResponse = Vec<UpcomingChargeItem>;

// ===== Requests =====

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
    /// Day of month (1–31) for monthly/quarterly/yearly.
    #[validate(range(min = 1, max = 31))]
    pub billing_day: i16,
    pub next_charge_date: Date,
}

#[derive(Deserialize, Debug, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSubscriptionRequest {
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
