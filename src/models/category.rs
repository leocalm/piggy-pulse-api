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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CategoryBehavior {
    Fixed,
    Variable,
    Subscription,
}

pub fn category_behavior_from_db(s: &str) -> Option<CategoryBehavior> {
    match s {
        "fixed" => Some(CategoryBehavior::Fixed),
        "variable" => Some(CategoryBehavior::Variable),
        "subscription" => Some(CategoryBehavior::Subscription),
        _ => None,
    }
}

pub fn category_behavior_to_db(b: CategoryBehavior) -> &'static str {
    match b {
        CategoryBehavior::Fixed => "fixed",
        CategoryBehavior::Variable => "variable",
        CategoryBehavior::Subscription => "subscription",
    }
}

#[derive(Debug, Clone, Default)]
pub struct Category {
    pub id: Uuid,
    pub name: String,
    pub color: String,
    pub icon: String,
    pub parent_id: Option<Uuid>,
    pub category_type: CategoryType,
    pub is_archived: bool,
    pub description: Option<String>,
    pub is_system: bool,
    pub behavior: Option<CategoryBehavior>,
}

#[derive(Deserialize, Debug, Validate, JsonSchema)]
pub struct CategoryRequest {
    #[validate(length(min = 3))]
    pub name: String,
    #[validate(length(min = 3))]
    pub color: String,
    pub icon: String,
    pub parent_id: Option<Uuid>,
    pub category_type: CategoryType,
    pub description: Option<String>,
    #[serde(default)]
    pub behavior: Option<String>,
}

#[derive(Serialize, Debug, Clone, JsonSchema)]
pub struct CategoryResponse {
    pub id: Uuid,
    pub name: String,
    pub color: String,
    pub icon: String,
    pub parent_id: Option<Uuid>,
    pub category_type: CategoryType,
    pub is_archived: bool,
    pub description: Option<String>,
    pub is_system: bool,
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
            is_archived: category.is_archived,
            description: category.description.clone(),
            is_system: category.is_system,
        }
    }
}
