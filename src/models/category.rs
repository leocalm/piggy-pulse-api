use chrono::{DateTime, Utc};
use rocket::serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use uuid::Uuid;
use validator::Validate;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Eq, PartialEq, Default, JsonSchema)]
pub enum CategoryType {
    Incoming,
    #[default]
    Outgoing,
    Transfer,
}

#[derive(Serialize, Debug, Clone, Default)]
pub struct Category {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub color: String,
    pub icon: String,
    pub parent_id: Option<Uuid>,
    pub category_type: CategoryType,
    pub created_at: DateTime<Utc>,
}

#[derive(Deserialize, Debug, Validate, JsonSchema)]
pub struct CategoryRequest {
    #[validate(length(min = 3))]
    pub name: String,
    #[validate(length(min = 3))]
    pub color: String,
    #[validate(length(min = 3))]
    pub icon: String,
    pub parent_id: Option<Uuid>,
    pub category_type: CategoryType,
}

#[derive(Serialize, Debug, JsonSchema)]
pub struct CategoryResponse {
    pub id: Uuid,
    pub name: String,
    pub color: String,
    pub icon: String,
    pub parent_id: Option<Uuid>,
    pub category_type: CategoryType,
}

impl From<&Category> for CategoryResponse {
    fn from(category: &Category) -> Self {
        Self {
            id: category.id,
            name: category.name.clone(),
            color: category.color.clone(),
            icon: category.icon.clone(),
            parent_id: category.parent_id,
            category_type: category.category_type,
        }
    }
}
