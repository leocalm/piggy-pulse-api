use crate::models::dashboard::{BudgetStabilityPeriodResponse, PeriodContextSummaryResponse};
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
}

#[derive(Serialize, Debug, Clone, JsonSchema)]
pub struct CategoryOption {
    pub id: Uuid,
    pub name: String,
    pub icon: String,
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

#[derive(Debug, Clone)]
pub struct CategoryWithStats {
    pub category: Category,
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

#[derive(Debug, Clone)]
pub struct CategoryBudgetedDiagnosticsRow {
    pub category: Category,
    pub budgeted_value: i32,
    pub actual_value: i64,
    pub variance_value: i64,
    /// Progress in basis points (percent * 100). Example: 12_500 = 125.00%.
    pub progress_basis_points: i32,
    pub recent_closed_periods: Vec<BudgetStabilityPeriodResponse>,
}

#[derive(Serialize, Debug, Clone, JsonSchema)]
pub struct CategoryBudgetedDiagnosticsRowResponse {
    #[serde(flatten)]
    pub category: CategoryResponse,
    pub budgeted_value: i32,
    pub actual_value: i64,
    pub variance_value: i64,
    /// Progress in basis points (percent * 100). Example: 12_500 = 125.00%.
    pub progress_basis_points: i32,
    pub recent_closed_periods: Vec<BudgetStabilityPeriodResponse>,
}

#[derive(Debug, Clone)]
pub struct CategoryUnbudgetedDiagnosticsRow {
    pub category: Category,
    pub actual_value: i64,
    /// Share in basis points (percent * 100). Example: 2_500 = 25.00%.
    pub share_of_total_basis_points: i32,
}

#[derive(Serialize, Debug, Clone, JsonSchema)]
pub struct CategoryUnbudgetedDiagnosticsRowResponse {
    #[serde(flatten)]
    pub category: CategoryResponse,
    pub actual_value: i64,
    /// Share in basis points (percent * 100). Example: 2_500 = 25.00%.
    pub share_of_total_basis_points: i32,
}

#[derive(Debug, Clone)]
pub struct CategoriesDiagnostics {
    pub period_summary: PeriodContextSummaryResponse,
    pub budgeted_rows: Vec<CategoryBudgetedDiagnosticsRow>,
    pub unbudgeted_rows: Vec<CategoryUnbudgetedDiagnosticsRow>,
}

#[derive(Serialize, Debug, Clone, JsonSchema)]
pub struct CategoriesDiagnosticsResponse {
    pub period_summary: PeriodContextSummaryResponse,
    pub budgeted_rows: Vec<CategoryBudgetedDiagnosticsRowResponse>,
    pub unbudgeted_rows: Vec<CategoryUnbudgetedDiagnosticsRowResponse>,
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
        }
    }
}

impl From<&Category> for CategoryOption {
    fn from(category: &Category) -> Self {
        Self {
            id: category.id,
            name: category.name.clone(),
            icon: category.icon.clone(),
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

impl From<&CategoryBudgetedDiagnosticsRow> for CategoryBudgetedDiagnosticsRowResponse {
    fn from(row: &CategoryBudgetedDiagnosticsRow) -> Self {
        Self {
            category: (&row.category).into(),
            budgeted_value: row.budgeted_value,
            actual_value: row.actual_value,
            variance_value: row.variance_value,
            progress_basis_points: row.progress_basis_points,
            recent_closed_periods: row.recent_closed_periods.clone(),
        }
    }
}

impl From<&CategoryUnbudgetedDiagnosticsRow> for CategoryUnbudgetedDiagnosticsRowResponse {
    fn from(row: &CategoryUnbudgetedDiagnosticsRow) -> Self {
        Self {
            category: (&row.category).into(),
            actual_value: row.actual_value,
            share_of_total_basis_points: row.share_of_total_basis_points,
        }
    }
}

impl From<&CategoriesDiagnostics> for CategoriesDiagnosticsResponse {
    fn from(value: &CategoriesDiagnostics) -> Self {
        Self {
            period_summary: value.period_summary.clone(),
            budgeted_rows: value.budgeted_rows.iter().map(CategoryBudgetedDiagnosticsRowResponse::from).collect(),
            unbudgeted_rows: value.unbudgeted_rows.iter().map(CategoryUnbudgetedDiagnosticsRowResponse::from).collect(),
        }
    }
}

/// Category row for the management view with global transaction count and children info.
#[derive(Debug, Clone)]
pub struct CategoryManagementRow {
    pub category: Category,
    /// Global transaction count (all time, not period-scoped)
    pub global_transaction_count: i64,
    /// Number of active children (for archive blocking logic)
    pub active_children_count: i64,
}

#[derive(Serialize, Debug, Clone, JsonSchema)]
#[schemars(example = "category_management_response_example")]
pub struct CategoryManagementResponse {
    #[serde(flatten)]
    pub category: CategoryResponse,
    /// Global transaction count (all time, not period-scoped)
    pub global_transaction_count: i64,
    /// Number of active children
    pub active_children_count: i64,
}

impl From<&CategoryManagementRow> for CategoryManagementResponse {
    fn from(row: &CategoryManagementRow) -> Self {
        Self {
            category: (&row.category).into(),
            global_transaction_count: row.global_transaction_count,
            active_children_count: row.active_children_count,
        }
    }
}

/// Response for the categories management list endpoint.
#[derive(Serialize, Debug, Clone, JsonSchema)]
pub struct CategoriesManagementListResponse {
    pub incoming: Vec<CategoryManagementResponse>,
    pub outgoing: Vec<CategoryManagementResponse>,
    pub archived: Vec<CategoryManagementResponse>,
}

fn category_management_response_example() -> serde_json::Value {
    json!({
        "id": "d2719f56-2b88-4b7a-b7c1-0b6b92d5c4d4",
        "name": "Dining",
        "color": "#FF6B6B",
        "icon": "🍽️",
        "parent_id": null,
        "category_type": "Outgoing",
        "is_archived": false,
        "description": "Meals outside home and food delivery",
        "global_transaction_count": 21,
        "active_children_count": 2
    })
}

pub fn difference_vs_average_percentage(used_in_period: i64, average_period_usage: i64) -> i32 {
    if average_period_usage <= 0 {
        0
    } else {
        let percent = (used_in_period.saturating_mul(100)) / average_period_usage;
        percent.clamp(i32::MIN as i64, i32::MAX as i64) as i32
    }
}

pub fn variance_value(actual_value: i64, budgeted_value: i32) -> i64 {
    actual_value.saturating_sub(i64::from(budgeted_value))
}

pub fn progress_basis_points(actual_value: i64, budgeted_value: i32) -> i32 {
    if budgeted_value <= 0 {
        return 0;
    }

    let safe_actual = actual_value.max(0);
    let points = safe_actual.saturating_mul(10_000) / i64::from(budgeted_value);
    points.clamp(i32::MIN as i64, i32::MAX as i64) as i32
}

pub fn share_of_total_basis_points(value: i64, total: i64) -> i32 {
    let safe_total = total.max(0);
    if safe_total == 0 {
        return 0;
    }

    let safe_value = value.max(0);
    let points = safe_value.saturating_mul(10_000) / safe_total;
    points.clamp(i32::MIN as i64, i32::MAX as i64) as i32
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
    use super::{difference_vs_average_percentage, progress_basis_points, share_of_total_basis_points, variance_value};

    #[test]
    fn test_difference_vs_average_percentage_zero_average() {
        assert_eq!(difference_vs_average_percentage(100, 0), 0);
    }

    #[test]
    fn test_difference_vs_average_percentage_basic() {
        assert_eq!(difference_vs_average_percentage(50, 100), 50);
        assert_eq!(difference_vs_average_percentage(150, 100), 150);
    }

    #[test]
    fn test_variance_value() {
        assert_eq!(variance_value(15_000, 10_000), 5_000);
        assert_eq!(variance_value(8_000, 10_000), -2_000);
    }

    #[test]
    fn test_progress_basis_points() {
        assert_eq!(progress_basis_points(12_500, 10_000), 12_500);
        assert_eq!(progress_basis_points(-50, 10_000), 0);
        assert_eq!(progress_basis_points(1_000, 0), 0);
    }

    #[test]
    fn test_share_of_total_basis_points() {
        assert_eq!(share_of_total_basis_points(250, 1_000), 2_500);
        assert_eq!(share_of_total_basis_points(-50, 1_000), 0);
        assert_eq!(share_of_total_basis_points(100, 0), 0);
    }
}
