#![allow(unused)]

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::dto::common::{Date, PaginatedResponse};
use crate::dto::transactions::TransactionResponse;

// ===== Enums =====

#[derive(Serialize, Deserialize, Debug, Copy, Clone, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum InclusionMode {
    Manual,
    Rules,
    All,
}

// ===== OverlayCategoryCap =====

#[derive(Serialize, Deserialize, Debug, Validate)]
#[serde(rename_all = "camelCase")]
pub struct OverlayCategoryCap {
    pub category_id: Uuid,
    #[validate(range(min = 0))]
    pub cap_amount: i64, // Cap in cents
}

// ===== OverlayRules =====

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct OverlayRules {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category_ids: Option<Vec<Uuid>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vendor_ids: Option<Vec<Uuid>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_ids: Option<Vec<Uuid>>,
}

// ===== OverlayResponse =====

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct OverlayResponse {
    pub id: Uuid,
    pub name: String,
    pub icon: Option<String>,
    pub start_date: Date,
    pub end_date: Date,
    pub inclusion_mode: InclusionMode,
    pub total_cap_amount: Option<i64>, // Optional total cap in cents
    pub spent_amount: i64,             // Total spent in cents
    pub transaction_count: i64,
    pub category_caps: Vec<OverlayCategoryCap>,
    pub rules: OverlayRules,
}

pub type OverlayListResponse = PaginatedResponse<OverlayResponse>;

// ===== OverlayTransactionMembership =====

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct OverlayTransactionMembership {
    pub is_included: bool,
    pub inclusion_source: Option<InclusionMode>,
}

// ===== OverlayTransactionResponse =====

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct OverlayTransactionResponse {
    #[serde(flatten)]
    pub transaction: TransactionResponse,
    pub membership: OverlayTransactionMembership,
}

pub type OverlayTransactionListResponse = PaginatedResponse<OverlayTransactionResponse>;

// ===== Requests =====

#[derive(Deserialize, Debug, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateOverlayRequest {
    #[validate(length(min = 1))]
    pub name: String,
    pub icon: Option<String>,
    pub start_date: Date,
    pub end_date: Date,
    pub inclusion_mode: InclusionMode,
    #[validate(range(min = 0))]
    pub total_cap_amount: Option<i64>,
    #[serde(default)]
    #[validate(nested)]
    pub category_caps: Vec<OverlayCategoryCap>,
    pub rules: Option<OverlayRules>,
}

pub type UpdateOverlayRequest = CreateOverlayRequest;
