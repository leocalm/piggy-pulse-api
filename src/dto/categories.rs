use std::sync::LazyLock;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as B64;
use regex::Regex;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::{Validate, ValidationError};

use crate::dto::common::PaginatedResponse;

static EMOJI_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\p{Emoji_Presentation}(\p{Emoji_Modifier}|\u{FE0F}|\u{20E3})?(\u{200D}\p{Emoji_Presentation}(\p{Emoji_Modifier}|\u{FE0F})?)*$").unwrap()
});

fn validate_emoji(value: &str) -> Result<(), ValidationError> {
    if !EMOJI_REGEX.is_match(value) {
        return Err(ValidationError::new("icon_must_be_emoji"));
    }
    Ok(())
}

fn b64(bytes: &[u8]) -> String {
    B64.encode(bytes)
}

// ===== Enums =====

#[derive(Serialize, Deserialize, Debug, Copy, Clone, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CategoryType {
    Income,
    Expense,
    Transfer,
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CategoryBehavior {
    Fixed,
    Variable,
    Subscription,
}

#[derive(Serialize, Debug, Copy, Clone, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CategoryStatus {
    Active,
    Inactive,
}

// ===== Encrypted response =====

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct EncryptedCategoryResponse {
    pub id: Uuid,
    #[serde(rename = "type")]
    pub category_type: CategoryType,
    pub behavior: Option<CategoryBehavior>,
    pub parent_id: Option<Uuid>,
    pub is_system: bool,
    pub status: CategoryStatus,
    pub name_enc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color_enc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_enc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description_enc: Option<String>,
}

pub type CategoryListResponse = PaginatedResponse<EncryptedCategoryResponse>;

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CategoryOptionResponse {
    pub id: Uuid,
    #[serde(rename = "type")]
    pub category_type: CategoryType,
    pub behavior: Option<CategoryBehavior>,
    pub name_enc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color_enc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_enc: Option<String>,
}

pub type CategoryOptionListResponse = Vec<CategoryOptionResponse>;

// ===== Requests =====

#[derive(Deserialize, Debug, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateCategoryRequest {
    #[validate(length(min = 1))]
    pub name: String,
    #[serde(rename = "type")]
    pub category_type: CategoryType,
    pub behavior: Option<CategoryBehavior>,
    #[validate(custom(function = "validate_emoji"))]
    pub icon: String,
    pub color: Option<String>,
    pub description: Option<String>,
    pub parent_id: Option<Uuid>,
}

pub type UpdateCategoryRequest = CreateCategoryRequest;

// ===== Targets (budget_category) =====
//
// Targets are stored per-category as a single budget_category row
// with an encrypted budgeted_value. No period scoping — the value is
// the user's standing target, used as-is by the client to render
// progress bars against any period.

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct EncryptedTargetResponse {
    pub id: Uuid,
    pub category_id: Uuid,
    pub is_excluded: bool,
    pub budgeted_value_enc: String,
}

pub type TargetListResponse = Vec<EncryptedTargetResponse>;

#[derive(Deserialize, Debug, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateTargetRequest {
    pub category_id: Uuid,
    #[validate(range(min = 0))]
    pub value: i64,
}

#[derive(Deserialize, Debug, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTargetRequest {
    #[validate(range(min = 0))]
    pub value: i64,
}

// ===== Conversions =====

use crate::models::category::{Category, CategoryBehavior as ModelBehavior, CategoryType as ModelType};

impl From<ModelType> for CategoryType {
    fn from(t: ModelType) -> Self {
        match t {
            ModelType::Incoming => CategoryType::Income,
            ModelType::Outgoing => CategoryType::Expense,
            ModelType::Transfer => CategoryType::Transfer,
        }
    }
}

impl From<CategoryType> for ModelType {
    fn from(t: CategoryType) -> Self {
        match t {
            CategoryType::Income => ModelType::Incoming,
            CategoryType::Expense => ModelType::Outgoing,
            CategoryType::Transfer => ModelType::Transfer,
        }
    }
}

impl From<ModelBehavior> for CategoryBehavior {
    fn from(b: ModelBehavior) -> Self {
        match b {
            ModelBehavior::Fixed => CategoryBehavior::Fixed,
            ModelBehavior::Variable => CategoryBehavior::Variable,
            ModelBehavior::Subscription => CategoryBehavior::Subscription,
        }
    }
}

impl From<CategoryBehavior> for ModelBehavior {
    fn from(b: CategoryBehavior) -> Self {
        match b {
            CategoryBehavior::Fixed => ModelBehavior::Fixed,
            CategoryBehavior::Variable => ModelBehavior::Variable,
            CategoryBehavior::Subscription => ModelBehavior::Subscription,
        }
    }
}

pub fn to_encrypted_response(category: &Category) -> EncryptedCategoryResponse {
    EncryptedCategoryResponse {
        id: category.id,
        category_type: category.category_type.into(),
        behavior: category.behavior.map(Into::into),
        parent_id: category.parent_id,
        is_system: category.is_system,
        status: if category.is_archived {
            CategoryStatus::Inactive
        } else {
            CategoryStatus::Active
        },
        name_enc: b64(&category.name_enc),
        color_enc: category.color_enc.as_deref().map(b64),
        icon_enc: category.icon_enc.as_deref().map(b64),
        description_enc: category.description_enc.as_deref().map(b64),
    }
}

pub fn to_option_response(category: &Category) -> CategoryOptionResponse {
    CategoryOptionResponse {
        id: category.id,
        category_type: category.category_type.into(),
        behavior: category.behavior.map(Into::into),
        name_enc: b64(&category.name_enc),
        color_enc: category.color_enc.as_deref().map(b64),
        icon_enc: category.icon_enc.as_deref().map(b64),
    }
}

pub fn target_to_response(id: Uuid, category_id: Uuid, is_excluded: bool, value_enc: &[u8]) -> EncryptedTargetResponse {
    EncryptedTargetResponse {
        id,
        category_id,
        is_excluded,
        budgeted_value_enc: b64(value_enc),
    }
}
