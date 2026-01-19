use chrono::{DateTime, Utc};
use rocket::serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Debug, Clone)]
pub struct Vendor {
    pub id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Deserialize, Debug)]
pub struct VendorRequest {
    pub name: String,
}

#[derive(Serialize, Debug)]
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
