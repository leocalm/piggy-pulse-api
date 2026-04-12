use rocket::serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct Vendor {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub archived: bool,
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
