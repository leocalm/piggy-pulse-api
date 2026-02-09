use crate::models::transaction::TransactionResponse;
use chrono::{DateTime, NaiveDate, Utc};
use rocket::serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use uuid::Uuid;
use validator::Validate;

// ===== Enums =====

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, sqlx::Type)]
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

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum InclusionSource {
    Manual,
    Rules,
    All,
}

// ===== Rules Models =====

#[derive(Serialize, Deserialize, Debug, Clone, Default, JsonSchema)]
pub struct OverlayRules {
    #[serde(default)]
    pub category_ids: Vec<Uuid>,
    #[serde(default)]
    pub vendor_ids: Vec<Uuid>,
    #[serde(default)]
    pub account_ids: Vec<Uuid>,
}

// ===== Category Cap Models =====

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
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
}

// ===== Request DTOs =====

#[derive(Deserialize, Debug, Validate, JsonSchema)]
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

// ===== Response DTOs =====

#[derive(Serialize, Debug, JsonSchema)]
pub struct OverlayResponse {
    pub id: Uuid,
    pub name: String,
    pub icon: Option<String>,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub inclusion_mode: InclusionMode,
    pub total_cap_amount: Option<i64>,
    pub spent_amount: i64,
    pub transaction_count: i64,
    pub category_caps: Vec<OverlayCategoryCap>,
    pub rules: OverlayRules,
}

impl From<&OverlayWithMetrics> for OverlayResponse {
    fn from(overlay_with_metrics: &OverlayWithMetrics) -> Self {
        Self {
            id: overlay_with_metrics.overlay.id,
            name: overlay_with_metrics.overlay.name.clone(),
            icon: overlay_with_metrics.overlay.icon.clone(),
            start_date: overlay_with_metrics.overlay.start_date,
            end_date: overlay_with_metrics.overlay.end_date,
            inclusion_mode: overlay_with_metrics.overlay.inclusion_mode,
            total_cap_amount: overlay_with_metrics.overlay.total_cap_amount,
            spent_amount: overlay_with_metrics.spent_amount,
            transaction_count: overlay_with_metrics.transaction_count,
            category_caps: overlay_with_metrics.category_caps.clone(),
            rules: overlay_with_metrics.overlay.rules.clone(),
        }
    }
}

// ===== Transaction Membership Models =====

#[derive(Serialize, Debug, Clone, JsonSchema)]
pub struct TransactionMembership {
    pub is_included: bool,
    pub inclusion_source: Option<InclusionSource>,
}

#[derive(Serialize, Debug, JsonSchema)]
pub struct TransactionWithMembership {
    #[serde(flatten)]
    pub transaction: TransactionResponse,
    pub membership: TransactionMembership,
}
