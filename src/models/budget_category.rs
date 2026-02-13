use crate::models::category::{Category, CategoryResponse};
use rocket::serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone, Default)]
pub struct BudgetCategory {
    pub id: Uuid,
    pub category_id: Uuid,
    pub budgeted_value: i32,
    pub category: Category,
}

#[derive(Deserialize, Debug, Validate, JsonSchema)]
pub struct BudgetCategoryRequest {
    pub category_id: Uuid,
    #[validate(range(min = 0))]
    pub budgeted_value: i32,
}

#[derive(Serialize, Debug, JsonSchema)]
pub struct BudgetCategoryResponse {
    pub id: Uuid,
    pub category_id: Uuid,
    pub budgeted_value: i32,
    pub category: CategoryResponse,
}

impl From<&BudgetCategory> for BudgetCategoryResponse {
    fn from(budget_category: &BudgetCategory) -> Self {
        Self {
            id: budget_category.id,
            category_id: budget_category.category_id,
            budgeted_value: budget_category.budgeted_value,
            category: CategoryResponse::from(&budget_category.category),
        }
    }
}
