use chrono::NaiveDate;
use rocket::serde::Serialize;
use schemars::JsonSchema;
use schemars::schema::{Metadata, Schema};
use serde_json::json;

#[derive(Serialize, Debug, JsonSchema)]
pub struct BudgetPerDayResponse {
    pub account_name: String,
    pub date: String,
    pub balance: i64,
}

#[derive(Serialize, Debug, Ord, PartialOrd, Eq, PartialEq, Clone, JsonSchema)]
pub struct SpentPerCategoryResponse {
    pub category_name: String,
    pub budgeted_value: i64,
    pub amount_spent: i64,
    /// Percentage spent in basis points (percent * 100). Example: 2534 = 25.34%.
    #[schemars(description = "Percentage spent in basis points (percent * 100). Example: 2534 = 25.34%.")]
    pub percentage_spent: i32,
}

#[derive(Serialize, Debug, JsonSchema)]
#[serde(transparent)]
#[schemars(schema_with = "spent_per_category_list_schema")]
pub struct SpentPerCategoryListResponse(pub Vec<SpentPerCategoryResponse>);

#[allow(dead_code)]
fn spent_per_category_list_schema(generator: &mut schemars::r#gen::SchemaGenerator) -> Schema {
    let mut schema = <Vec<SpentPerCategoryResponse>>::json_schema(generator);
    if let Schema::Object(ref mut schema_obj) = schema {
        let metadata = schema_obj.metadata.get_or_insert_with(|| Box::new(Metadata::default()));
        metadata.examples = vec![json!([
            {
                "category_name": "Groceries",
                "budgeted_value": 50000,
                "amount_spent": 12670,
                "percentage_spent": 2534
            },
            {
                "category_name": "Dining Out",
                "budgeted_value": 20000,
                "amount_spent": 17500,
                "percentage_spent": 8750
            }
        ])];
    }
    schema
}

#[derive(Serialize, Debug, sqlx::FromRow, JsonSchema)]
pub struct MonthlyBurnInResponse {
    pub total_budget: i64,
    pub spent_budget: i64,
    pub current_day: i32,
    pub days_in_period: i32,
}

#[derive(Serialize, Debug, JsonSchema)]
pub struct MonthProgressResponse {
    pub current_date: NaiveDate,
    pub days_in_period: u32,
    pub remaining_days: u32,
    pub days_passed_percentage: u32,
}

#[derive(Serialize, Debug, JsonSchema)]
pub struct TotalAssetsResponse {
    pub total_assets: i64,
}

#[derive(Serialize, Debug, JsonSchema)]
pub struct NetPositionResponse {
    pub total_net_position: i64,
    pub change_this_period: i64,
    pub liquid_balance: i64,
    pub protected_balance: i64,
    pub debt_balance: i64,
    pub account_count: i64,
}
