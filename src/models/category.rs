use chrono::{DateTime, Utc};
use rocket::serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use serde_json::json;
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

#[derive(Serialize, Debug, Clone, JsonSchema)]
pub struct CategoryResponse {
    pub id: Uuid,
    pub name: String,
    pub color: String,
    pub icon: String,
    pub parent_id: Option<Uuid>,
    pub category_type: CategoryType,
}

#[derive(Serialize, Debug, Clone, JsonSchema)]
pub struct CategoryStats {
    #[schemars(description = "Total amount used in the selected budget period.", example = "category_used_in_period_example")]
    pub used_in_period: i64,
    /// Percentage of usage in the selected period vs average period usage.
    #[schemars(
        description = "Percentage of usage in the selected period vs average period usage.",
        example = "category_difference_vs_average_example"
    )]
    pub difference_vs_average_percentage: i32,
    #[schemars(
        description = "Number of transactions in the selected budget period.",
        example = "category_transaction_count_example"
    )]
    pub transaction_count: i64,
}

#[derive(Serialize, Debug, Clone)]
pub struct CategoryWithStats {
    #[serde(flatten)]
    pub category: Category,

    #[serde(flatten)]
    pub stats: CategoryStats,
}

#[derive(Serialize, Debug, Clone, JsonSchema)]
#[schemars(example = "category_with_stats_response_example")]
pub struct CategoryWithStatsResponse {
    #[serde(flatten)]
    pub category: CategoryResponse,

    #[serde(flatten)]
    pub stats: CategoryStats,
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

impl From<&CategoryWithStats> for CategoryWithStatsResponse {
    fn from(category_with_stats: &CategoryWithStats) -> Self {
        Self {
            category: (&category_with_stats.category).into(),
            stats: category_with_stats.stats.clone(),
        }
    }
}

pub fn difference_vs_average_percentage(used_in_period: i64, average_period_usage: i64) -> i32 {
    if average_period_usage <= 0 {
        0
    } else {
        let percent = (used_in_period.saturating_mul(100)) / average_period_usage;
        percent.clamp(i32::MIN as i64, i32::MAX as i64) as i32
    }
}

fn category_used_in_period_example() -> i64 {
    12_670
}

fn category_difference_vs_average_example() -> i32 {
    125
}

fn category_transaction_count_example() -> i64 {
    7
}

fn category_with_stats_response_example() -> serde_json::Value {
    json!({
        "id": "d2719f56-2b88-4b7a-b7c1-0b6b92d5c4d4",
        "name": "Groceries",
        "color": "#00FF00",
        "icon": "cart",
        "parent_id": null,
        "category_type": "Outgoing",
        "used_in_period": 12670,
        "difference_vs_average_percentage": 125,
        "transaction_count": 7
    })
}

#[cfg(test)]
mod tests {
    use super::difference_vs_average_percentage;

    #[test]
    fn test_difference_vs_average_percentage_zero_average() {
        assert_eq!(difference_vs_average_percentage(100, 0), 0);
    }

    #[test]
    fn test_difference_vs_average_percentage_basic() {
        assert_eq!(difference_vs_average_percentage(50, 100), 50);
        assert_eq!(difference_vs_average_percentage(150, 100), 150);
    }
}
