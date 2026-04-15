use base64::Engine;
use base64::engine::general_purpose::STANDARD as B64;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::dto::common::PaginatedResponse;
use crate::models::vendor::Vendor;

fn b64(bytes: &[u8]) -> String {
    B64.encode(bytes)
}

#[derive(Serialize, Debug, Copy, Clone, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum VendorStatus {
    Active,
    Inactive,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct EncryptedVendorResponse {
    pub id: Uuid,
    pub status: VendorStatus,
    pub name_enc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description_enc: Option<String>,
}

pub type VendorListResponse = PaginatedResponse<EncryptedVendorResponse>;

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct VendorOptionResponse {
    pub id: Uuid,
    pub name_enc: String,
}

pub type VendorOptionListResponse = Vec<VendorOptionResponse>;

#[derive(Deserialize, Debug, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateVendorRequest {
    #[validate(length(min = 1))]
    pub name: String,
    pub description: Option<String>,
}

pub type UpdateVendorRequest = CreateVendorRequest;

pub fn to_encrypted_response(vendor: &Vendor) -> EncryptedVendorResponse {
    EncryptedVendorResponse {
        id: vendor.id,
        status: if vendor.archived { VendorStatus::Inactive } else { VendorStatus::Active },
        name_enc: b64(&vendor.name_enc),
        description_enc: vendor.description_enc.as_deref().map(b64),
    }
}

pub fn to_option_response(vendor: &Vendor) -> VendorOptionResponse {
    VendorOptionResponse {
        id: vendor.id,
        name_enc: b64(&vendor.name_enc),
    }
}
