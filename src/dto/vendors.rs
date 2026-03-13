#![allow(unused)]

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::dto::common::{PaginatedResponse, VendorMinimal};

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
}

pub type VendorListResponse = PaginatedResponse<VendorSummaryResponse>;

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
