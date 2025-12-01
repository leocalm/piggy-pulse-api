use crate::models::category::{Category, CategoryResponse};
use chrono::{DateTime, Utc};
use rocket::serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Debug)]
pub struct BudgetCategory {
    pub id: Uuid,
    pub category_id: Uuid,
    pub budgeted_value: u32,
    pub created_at: DateTime<Utc>,
    pub deleted: bool,
    pub deleted_at: Option<DateTime<Utc>>,
    pub category: Category,
}

#[derive(Deserialize, Debug)]
pub struct BudgetCategoryRequest {
    pub category_id: Uuid,
    pub budgeted_value: u32,
}

#[derive(Serialize, Debug)]
pub struct BudgetCategoryResponse {
    pub id: Uuid,
    pub category_id: Uuid,
    pub budgeted_value: u32,
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
