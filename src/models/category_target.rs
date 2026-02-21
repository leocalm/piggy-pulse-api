use rocket::serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use uuid::Uuid;
use validator::Validate;

/// A single row in the category targets view
#[derive(Serialize, Debug, JsonSchema)]
pub struct CategoryTargetRow {
    pub id: String,
    pub category_id: Uuid,
    pub category_name: String,
    pub category_type: String,
    pub category_icon: String,
    pub category_color: String,
    pub is_archived: bool,
    pub is_parent: bool,
    pub parent_category_name: Option<String>,
    pub current_target: Option<i32>,
    pub previous_target: Option<i32>,
    pub is_excluded: bool,
    pub exclusion_reason: Option<String>,
    pub projected_variance_basis_points: Option<i32>,
}

/// Full response for the category targets page
#[derive(Serialize, Debug, JsonSchema)]
pub struct CategoryTargetsResponse {
    pub period_id: Uuid,
    pub period_name: String,
    pub period_start_date: String,
    pub period_end_date: String,
    pub period_progress_percent: i32,
    pub total_targeted: i64,
    pub total_categories: i32,
    pub targeted_categories: i32,
    pub outgoing_targets: Vec<CategoryTargetRow>,
    pub incoming_targets: Vec<CategoryTargetRow>,
    pub excluded_categories: Vec<CategoryTargetRow>,
}

/// A single target entry in a batch upsert request
#[derive(Deserialize, Debug, Validate, JsonSchema)]
pub struct TargetEntry {
    pub category_id: Uuid,
    #[validate(range(min = 0))]
    pub budgeted_value: i32,
}

/// Batch upsert request for saving targets
#[derive(Deserialize, Debug, Validate, JsonSchema)]
pub struct BatchUpsertTargetsRequest {
    #[validate(nested)]
    pub targets: Vec<TargetEntry>,
}
