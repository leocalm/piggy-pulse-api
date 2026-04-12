use crate::models::transaction::TransactionResponse;
use chrono::{DateTime, NaiveDate, Utc};
use rocket::serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

// ===== Enums =====

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "text", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum InclusionMode {
    Manual,
    Rules,
    All,
}

impl std::fmt::Display for InclusionMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InclusionMode::Manual => write!(f, "manual"),
            InclusionMode::Rules => write!(f, "rules"),
            InclusionMode::All => write!(f, "all"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum InclusionSource {
    Manual,
    Rules,
    All,
}

// ===== Rules Models =====

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct OverlayRules {
    #[serde(default)]
    pub category_ids: Vec<Uuid>,
    #[serde(default)]
    pub vendor_ids: Vec<Uuid>,
    #[serde(default)]
    pub account_ids: Vec<Uuid>,
}

// ===== Category Cap Models =====

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OverlayCategoryCap {
    pub category_id: Uuid,
    pub cap_amount: i64,
}

// ===== Overlay Domain Model =====

#[derive(Serialize, Debug, Clone)]
pub struct Overlay {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub icon: Option<String>,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub inclusion_mode: InclusionMode,
    pub total_cap_amount: Option<i64>,
    pub rules: OverlayRules,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct OverlayWithMetrics {
    pub overlay: Overlay,
    pub spent_amount: i64,
    pub transaction_count: i64,
    pub category_caps: Vec<OverlayCategoryCap>,
    pub category_breakdown: Vec<(Uuid, String, i64)>,
}

// ===== Request DTOs =====

#[derive(Deserialize, Debug, Validate)]
#[validate(schema(function = "validate_overlay_date_range"))]
pub struct OverlayRequest {
    #[validate(length(min = 1))]
    pub name: String,
    pub icon: Option<String>,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub inclusion_mode: InclusionMode,
    pub total_cap_amount: Option<i64>,
    #[serde(default)]
    pub category_caps: Vec<OverlayCategoryCap>,
    #[serde(default)]
    pub rules: OverlayRules,
}

fn validate_overlay_date_range(request: &OverlayRequest) -> Result<(), validator::ValidationError> {
    if request.start_date >= request.end_date {
        return Err(validator::ValidationError::new("start_date_must_be_before_end_date"));
    }
    Ok(())
}

// ===== Transaction Membership Models =====

#[derive(Serialize, Debug, Clone)]
pub struct TransactionMembership {
    pub is_included: bool,
    pub inclusion_source: Option<InclusionSource>,
}

#[derive(Serialize, Debug)]
pub struct TransactionWithMembership {
    #[serde(flatten)]
    pub transaction: TransactionResponse,
    pub membership: TransactionMembership,
}
