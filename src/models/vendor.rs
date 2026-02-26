use chrono::NaiveDate;
use rocket::serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use serde_json::json;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct Vendor {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub archived: bool,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct VendorStats {
    pub transaction_count: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_used_at: Option<NaiveDate>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct VendorPeriodStats {
    #[schemars(
        description = "Number of transactions in the selected budget period.",
        example = "vendor_period_transaction_count_example"
    )]
    pub transaction_count: i64,
    #[schemars(description = "Last used date (any time).", example = "vendor_last_used_example")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_used_at: Option<NaiveDate>,
}

#[derive(Deserialize, Debug, Validate, JsonSchema)]
pub struct VendorRequest {
    #[validate(length(min = 3))]
    pub name: String,
    #[validate(length(max = 500))]
    pub description: Option<String>,
}

#[derive(Serialize, Debug, Clone, JsonSchema)]
pub struct VendorResponse {
    pub id: Uuid,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub archived: bool,
}

impl From<&Vendor> for VendorResponse {
    fn from(vendor: &Vendor) -> Self {
        Self {
            id: vendor.id,
            name: vendor.name.clone(),
            description: vendor.description.clone(),
            archived: vendor.archived,
        }
    }
}

#[derive(Debug, Clone)]
pub struct VendorWithStats {
    pub vendor: Vendor,
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

#[derive(Serialize, Debug, Clone, JsonSchema)]
pub struct VendorWithStatsResponse {
    #[serde(flatten)]
    pub vendor: VendorResponse,

    #[serde(flatten)]
    pub stats: VendorStats,
}

#[derive(Debug, Clone)]
pub struct VendorWithPeriodStats {
    pub vendor: Vendor,
    pub stats: VendorPeriodStats,
}

impl From<&VendorWithPeriodStats> for VendorWithPeriodStatsResponse {
    fn from(vendor_with_status: &VendorWithPeriodStats) -> Self {
        Self {
            vendor: (&vendor_with_status.vendor).into(),
            stats: vendor_with_status.stats.clone(),
        }
    }
}

#[derive(Serialize, Debug, Clone, JsonSchema)]
#[schemars(example = "vendor_with_period_stats_example")]
pub struct VendorWithPeriodStatsResponse {
    #[serde(flatten)]
    pub vendor: VendorResponse,

    #[serde(flatten)]
    pub stats: VendorPeriodStats,
}

fn vendor_period_transaction_count_example() -> i64 {
    3
}

fn vendor_last_used_example() -> NaiveDate {
    NaiveDate::from_ymd_opt(2024, 1, 14).expect("valid date")
}

fn vendor_with_period_stats_example() -> serde_json::Value {
    json!({
        "id": "6f64b6ea-1a79-41e4-95c2-86f8393c4b30",
        "name": "Vendor Co",
        "transaction_count": 3,
        "last_used_at": "2024-01-14"
    })
}
