use chrono::{DateTime, NaiveDate, Utc};
use rocket::serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Debug, Clone, Default)]
pub struct Vendor {
    pub id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct VendorStats {
    pub transaction_count: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_used_at: Option<NaiveDate>,
}

#[derive(Deserialize, Debug)]
pub struct VendorRequest {
    pub name: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct VendorResponse {
    pub id: Uuid,
    pub name: String,
}

impl From<&Vendor> for VendorResponse {
    fn from(vendor: &Vendor) -> Self {
        Self {
            id: vendor.id,
            name: vendor.name.clone(),
        }
    }
}

#[derive(Serialize, Debug, Clone)]
pub struct VendorWithStats {
    #[serde(flatten)]
    pub vendor: Vendor,

    #[serde(flatten)]
    pub stats: VendorStats,
}

impl From<&VendorWithStats> for VendorWithStatsResponse {
    fn from(vendor_with_status: &VendorWithStats) -> Self {
        Self {
            vendor: (&vendor_with_status.vendor).into(),
            stats: vendor_with_status.stats.clone(),
        }
    }
}

#[derive(Serialize, Debug, Clone)]
pub struct VendorWithStatsResponse {
    #[serde(flatten)]
    pub vendor: VendorResponse,

    #[serde(flatten)]
    pub stats: VendorStats,
}
