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
    pub used_this_month: i64,
    /// Percentage of usage this month vs average monthly usage.
    pub difference_vs_average_percentage: i32,
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

pub fn difference_vs_average_percentage(used_this_month: i64, average_monthly_usage: i64) -> i32 {
    if average_monthly_usage <= 0 {
        0
    } else {
        let percent = (used_this_month.saturating_mul(100)) / average_monthly_usage;
        percent.clamp(i32::MIN as i64, i32::MAX as i64) as i32
    }
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
