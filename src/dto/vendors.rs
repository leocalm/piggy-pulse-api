#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::dto::common::{Date, PaginatedResponse, VendorMinimal};

// ===== Enums =====

#[derive(Serialize, Debug, Copy, Clone, Eq, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum VendorStatus {
    #[default]
    Active,
    Inactive,
}

// ===== VendorBase =====

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct VendorBase {
    pub id: Uuid,
    pub name: String,
    pub status: VendorStatus,
}

// ===== VendorResponse =====

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct VendorResponse {
    #[serde(flatten)]
    pub base: VendorBase,
    pub description: Option<String>,
}

// ===== VendorSummaryResponse =====

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct VendorSummaryResponse {
    #[serde(flatten)]
    pub base: VendorResponse,
    pub number_of_transactions: i64,
    pub total_spend: i64,
}

pub type VendorListResponse = PaginatedResponse<VendorSummaryResponse>;

// ===== VendorDetailResponse =====

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct VendorTrendItem {
    pub period_id: Uuid,
    pub period_name: String,
    pub total_spend: i64,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct VendorTopCategoryItem {
    pub category_id: Uuid,
    pub category_name: String,
    pub total_spend: i64,
    pub percentage: f64,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct VendorTransactionItem {
    pub id: Uuid,
    pub date: Date,
    pub amount: i64,
    pub description: String,
    pub category_id: Option<Uuid>,
    pub category_name: Option<String>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct VendorDetailResponse {
    #[serde(flatten)]
    pub base: VendorResponse,
    pub period_spend: i64,
    pub transaction_count: i64,
    pub average_transaction_amount: i64,
    pub trend: Vec<VendorTrendItem>,
    pub top_categories: Vec<VendorTopCategoryItem>,
    pub recent_transactions: Vec<VendorTransactionItem>,
}

// ===== VendorStatsResponse =====

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct VendorStatsResponse {
    pub total_vendors: i64,
    pub total_spend_this_period: i64,
    pub avg_spend_per_vendor: i64,
}

// ===== VendorOptionResponse =====

pub type VendorOptionResponse = VendorMinimal;
pub type VendorOptionListResponse = Vec<VendorOptionResponse>;

// ===== Requests =====

#[derive(Deserialize, Debug, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateVendorRequest {
    #[validate(length(min = 3))]
    pub name: String,
    #[validate(length(max = 500))]
    pub description: Option<String>,
}

pub type UpdateVendorRequest = CreateVendorRequest;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MergeVendorRequest {
    pub target_vendor_id: Uuid,
}
