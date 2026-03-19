#![allow(dead_code)]

use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::{Validate, ValidationError};

use crate::dto::common::{Date, HEX_COLOR_REGEX, PaginatedResponse};

static EMOJI_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    // Matches a single emoji sequence: an Emoji_Presentation char optionally followed by
    // a skin-tone modifier or variation selector, and zero or more ZWJ-joined pairs.
    //
    // Known limitations (acceptable for decorative category icons):
    // - Regional indicator flag sequences (e.g. 🇺🇸 = \p{Regional_Indicator}{2}) are not matched
    //   because each Regional_Indicator character lacks the Emoji_Presentation property on its own.
    // - Keycap sequences that start with an ASCII digit or symbol (e.g. 1️⃣ = [0-9#*]\uFE0F\u20E3)
    //   are not matched because their first character is plain ASCII, not Emoji_Presentation.
    Regex::new(r"^\p{Emoji_Presentation}(\p{Emoji_Modifier}|\u{FE0F}|\u{20E3})?(\u{200D}\p{Emoji_Presentation}(\p{Emoji_Modifier}|\u{FE0F})?)*$").unwrap()
});

fn validate_emoji(value: &str) -> Result<(), ValidationError> {
    if !EMOJI_REGEX.is_match(value) {
        return Err(ValidationError::new("icon_must_be_emoji"));
    }
    Ok(())
}

// ===== Enums =====

#[derive(Serialize, Deserialize, Debug, Copy, Clone, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CategoryType {
    Income,
    Expense,
    Transfer,
}

#[derive(Serialize, Debug, Copy, Clone, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CategoryStatus {
    Active,
    Inactive,
}

#[derive(Serialize, Debug, Copy, Clone, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TargetStatus {
    Active,
    Excluded,
}

// ===== CategoryBase =====

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CategoryBase {
    pub id: Uuid,
    pub name: String,
    #[serde(rename = "type")]
    pub category_type: CategoryType,
    pub icon: String,
    pub color: String,
    pub parent_id: Option<Uuid>,
    pub status: CategoryStatus,
}

// ===== CategoryResponse =====

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CategoryResponse {
    #[serde(flatten)]
    pub base: CategoryBase,
    pub description: Option<String>,
}

// ===== CategoryManagementListItem / Response =====

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CategoryManagementListItem {
    #[serde(flatten)]
    pub base: CategoryBase,
    pub description: Option<String>,
    pub number_of_transactions: i64,
}

pub type CategoryManagementListResponse = PaginatedResponse<CategoryManagementListItem>;

// ===== CategorySummaryItem =====

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CategorySummaryItem {
    #[serde(flatten)]
    pub base: CategoryBase,
    pub actual: i64,
    pub projected: i64,
    pub budgeted: Option<i64>,
    pub variance: i64,
}

// ===== CategoryOverview =====

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CategoryOverviewSummary {
    pub period_name: String,
    pub period_elapsed_percent: i64,
    pub total_spent: i64,
    pub total_budgeted: Option<i64>,
    pub variance: i64,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CategoryOverviewResponse {
    pub summary: CategoryOverviewSummary,
    pub categories: Vec<CategorySummaryItem>,
}

// ===== CategoryOptionResponse =====

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CategoryOptionResponse {
    pub id: Uuid,
    pub name: String,
    pub icon: String,
    pub color: String,
}

pub type CategoryOptionListResponse = Vec<CategoryOptionResponse>;

// ===== Category Requests =====

#[derive(Deserialize, Debug, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateCategoryRequest {
    #[validate(length(min = 3))]
    pub name: String,
    #[serde(rename = "type")]
    pub category_type: CategoryType,
    #[validate(custom(function = "validate_emoji"))]
    pub icon: String,
    #[validate(regex(path = *HEX_COLOR_REGEX))]
    pub color: String,
    pub description: Option<String>,
    pub parent_id: Option<Uuid>,
}

pub type UpdateCategoryRequest = CreateCategoryRequest;

// ===== Type conversion helpers =====

use crate::models::category::CategoryType as V1CategoryType;

impl CategoryType {
    pub fn to_v1(self) -> V1CategoryType {
        match self {
            CategoryType::Income => V1CategoryType::Incoming,
            CategoryType::Expense => V1CategoryType::Outgoing,
            CategoryType::Transfer => V1CategoryType::Transfer,
        }
    }

    pub fn from_v1(ct: V1CategoryType) -> Self {
        match ct {
            V1CategoryType::Incoming => CategoryType::Income,
            V1CategoryType::Outgoing => CategoryType::Expense,
            V1CategoryType::Transfer => CategoryType::Transfer,
        }
    }
}

impl CategoryStatus {
    pub fn from_archived(is_archived: bool) -> Self {
        if is_archived { CategoryStatus::Inactive } else { CategoryStatus::Active }
    }
}

impl CategoryBase {
    pub fn from_model(c: &crate::models::category::Category) -> Self {
        CategoryBase {
            id: c.id,
            name: c.name.clone(),
            category_type: CategoryType::from_v1(c.category_type),
            icon: c.icon.clone(),
            color: c.color.clone(),
            parent_id: c.parent_id,
            status: CategoryStatus::from_archived(c.is_archived),
        }
    }
}

impl CategoryResponse {
    pub fn from_model(c: &crate::models::category::Category) -> Self {
        CategoryResponse {
            base: CategoryBase::from_model(c),
            description: c.description.clone(),
        }
    }
}

// ===== TargetItem =====

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TargetItem {
    pub id: Uuid,
    pub name: String,
    #[serde(rename = "type")]
    pub category_type: CategoryType,
    pub parent_id: Option<Uuid>,
    pub previous_target: Option<i64>,
    pub current_target: Option<i64>,
    pub projected_variance: i64,
    pub status: TargetStatus,
    pub spent_in_period: i64,
}

// ===== TargetSummary =====

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CategoriesWithTargets {
    pub with_targets: i64,
    pub total: i64,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TargetSummary {
    pub period_name: String,
    pub period_start: Date,
    pub period_end: Option<Date>,
    pub current_position: i64,
    pub categories_with_targets: CategoriesWithTargets,
    pub period_progress: i64,
}

// ===== CategoryTargetsResponse =====

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CategoryTargetsResponse {
    pub summary: TargetSummary,
    pub targets: Vec<TargetItem>,
}

// ===== Target Requests =====

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
