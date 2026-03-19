use crate::database::postgres_repository::PostgresRepository;
use crate::dto::categories::{
    CategoriesWithTargets, CategoryBase, CategoryManagementListItem, CategoryManagementListResponse, CategoryOptionListResponse, CategoryOptionResponse,
    CategoryOverviewResponse, CategoryOverviewSummary, CategoryResponse, CategoryStatus, CategorySummaryItem, CategoryTargetsResponse, CategoryType,
    CreateTargetRequest, TargetItem, TargetStatus, TargetSummary,
};
use crate::dto::common::{Date, PaginatedResponse};
use crate::error::app_error::AppError;
use crate::models::category::{Category, CategoryType as V1CategoryType};
use uuid::Uuid;

pub struct CategoryService<'a> {
    repository: &'a PostgresRepository,
}

fn to_v2_category_type(ct: V1CategoryType) -> CategoryType {
    match ct {
        V1CategoryType::Incoming => CategoryType::Income,
        V1CategoryType::Outgoing => CategoryType::Expense,
        V1CategoryType::Transfer => CategoryType::Transfer,
    }
}

fn to_v2_status(is_archived: bool) -> CategoryStatus {
    if is_archived { CategoryStatus::Inactive } else { CategoryStatus::Active }
}

fn category_to_base(c: &Category) -> CategoryBase {
    CategoryBase {
        id: c.id,
        name: c.name.clone(),
        category_type: to_v2_category_type(c.category_type),
        icon: c.icon.clone(),
        color: c.color.clone(),
        parent_id: c.parent_id,
        status: to_v2_status(c.is_archived),
    }
}

fn category_to_response(c: &Category) -> CategoryResponse {
    CategoryResponse {
        base: category_to_base(c),
        description: c.description.clone(),
    }
}

impl<'a> CategoryService<'a> {
    pub fn new(repository: &'a PostgresRepository) -> Self {
        CategoryService { repository }
    }

    // ===== Categories =====

    pub async fn list_categories_v2(&self, cursor: Option<Uuid>, limit: i64, user_id: &Uuid) -> Result<CategoryManagementListResponse, AppError> {
        let (mut rows, total_count) = self.repository.list_categories_v2(cursor, limit, user_id).await?;

        let has_more = rows.len() as i64 > limit;
        if has_more {
            rows.truncate(limit as usize);
        }
        let next_cursor = if has_more { rows.last().map(|(c, _)| c.id.to_string()) } else { None };

        let data: Vec<CategoryManagementListItem> = rows
            .iter()
            .map(|(cat, tx_count)| CategoryManagementListItem {
                base: category_to_base(cat),
                description: cat.description.clone(),
                number_of_transactions: *tx_count,
            })
            .collect();

        Ok(PaginatedResponse {
            data,
            total_count,
            has_more,
            next_cursor,
        })
    }

    #[allow(dead_code)]
    pub async fn get_category_response(&self, id: &Uuid, user_id: &Uuid) -> Result<CategoryResponse, AppError> {
        let cat = self
            .repository
            .get_category_by_id(id, user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Category not found".to_string()))?;
        Ok(category_to_response(&cat))
    }

    pub async fn list_category_options(&self, user_id: &Uuid) -> Result<CategoryOptionListResponse, AppError> {
        let categories = self.repository.list_all_categories(user_id).await?;
        Ok(categories
            .iter()
            .filter(|c| !c.is_archived)
            .map(|c| CategoryOptionResponse {
                id: c.id,
                name: c.name.clone(),
                icon: c.icon.clone(),
                color: c.color.clone(),
            })
            .collect())
    }

    pub async fn get_category_overview(&self, period_id: &Uuid, user_id: &Uuid) -> Result<CategoryOverviewResponse, AppError> {
        let period = self.repository.get_budget_period(period_id, user_id).await?;

        let today = chrono::Utc::now().date_naive();
        let total_days = (period.end_date - period.start_date).num_days().max(1);
        let elapsed_days = (today - period.start_date).num_days().clamp(0, total_days);
        let period_elapsed_percent = ((elapsed_days * 100) / total_days) as i64;

        let category_data = self
            .repository
            .get_category_overview_data(period_id, &period.start_date, &period.end_date, user_id)
            .await?;

        let mut total_spent: i64 = 0;
        let mut total_budgeted: Option<i64> = None;

        let categories: Vec<CategorySummaryItem> = category_data
            .iter()
            .map(|row| {
                let actual = row.actual;
                let budgeted = row.budgeted;

                // Only count expense categories toward totals
                if row.category.category_type == V1CategoryType::Outgoing {
                    total_spent += actual;
                    if let Some(b) = budgeted {
                        *total_budgeted.get_or_insert(0) += b;
                    }
                }

                let projected = if period_elapsed_percent > 0 {
                    (actual * 100) / period_elapsed_percent
                } else {
                    0
                };

                let variance = budgeted.map_or(0, |b| b - actual);

                CategorySummaryItem {
                    base: category_to_base(&row.category),
                    actual,
                    projected,
                    budgeted,
                    variance,
                }
            })
            .collect();

        let variance = total_budgeted.map_or(0, |b| b - total_spent);

        Ok(CategoryOverviewResponse {
            summary: CategoryOverviewSummary {
                period_name: period.name,
                period_elapsed_percent,
                total_spent,
                total_budgeted,
                variance,
            },
            categories,
        })
    }

    pub async fn archive_category(&self, id: &Uuid, user_id: &Uuid) -> Result<CategoryResponse, AppError> {
        // Check current state first
        let cat = self
            .repository
            .get_category_by_id(id, user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Category not found".to_string()))?;

        if cat.is_archived {
            return Err(AppError::Conflict("Category is already archived".to_string()));
        }

        let archived = self.repository.archive_category(id, user_id).await?;
        Ok(category_to_response(&archived))
    }

    pub async fn unarchive_category(&self, id: &Uuid, user_id: &Uuid) -> Result<CategoryResponse, AppError> {
        let cat = self
            .repository
            .get_category_by_id(id, user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Category not found".to_string()))?;

        if !cat.is_archived {
            return Err(AppError::Conflict("Category is not archived".to_string()));
        }

        let restored = self.repository.restore_category(id, user_id).await?;
        Ok(category_to_response(&restored))
    }

    // ===== Targets =====

    pub async fn list_targets(&self, period_id: &Uuid, user_id: &Uuid) -> Result<CategoryTargetsResponse, AppError> {
        let period = self.repository.get_budget_period(period_id, user_id).await?;

        let today = chrono::Utc::now().date_naive();
        let total_days = (period.end_date - period.start_date).num_days().max(1);
        let elapsed_days = (today - period.start_date).num_days().clamp(0, total_days);
        let period_progress = ((elapsed_days * 100) / total_days) as i64;

        let target_rows = self
            .repository
            .list_targets_v2(period_id, &period.start_date, &period.end_date, user_id)
            .await?;

        let mut with_targets: i64 = 0;
        let total = target_rows.len() as i64;
        let mut current_position: i64 = 0;

        let targets: Vec<TargetItem> = target_rows
            .iter()
            .map(|row| {
                if row.current_target.is_some() && !row.is_excluded {
                    with_targets += 1;
                }

                let spent = row.spent_in_period;
                let current_target = row.current_target;

                let projected_variance = match current_target {
                    Some(target) if target > 0 => {
                        let projected_spend = if period_progress > 0 { (spent * 100) / period_progress } else { 0 };
                        target - projected_spend
                    }
                    _ => 0,
                };

                if let Some(target) = current_target {
                    current_position += target - spent;
                }

                TargetItem {
                    id: row.target_id,
                    name: row.category_name.clone(),
                    category_type: to_v2_category_type(row.category_type),
                    parent_id: row.parent_id,
                    previous_target: row.previous_target,
                    current_target,
                    projected_variance,
                    status: if row.is_excluded { TargetStatus::Excluded } else { TargetStatus::Active },
                    spent_in_period: spent,
                }
            })
            .collect();

        Ok(CategoryTargetsResponse {
            summary: TargetSummary {
                period_name: period.name,
                period_start: Date(period.start_date),
                period_end: Some(Date(period.end_date)),
                current_position,
                categories_with_targets: CategoriesWithTargets { with_targets, total },
                period_progress,
            },
            targets,
        })
    }

    pub async fn create_target(&self, request: &CreateTargetRequest, user_id: &Uuid) -> Result<TargetItem, AppError> {
        // Verify category exists and belongs to user
        let cat = self
            .repository
            .get_category_by_id(&request.category_id, user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Category not found".to_string()))?;

        // Check if target already exists
        let existing = self.repository.get_target_for_category(&request.category_id, user_id).await?;
        if existing.is_some() {
            return Err(AppError::Conflict("Target already exists for this category".to_string()));
        }

        let target_id = self.repository.create_target(&request.category_id, request.value, user_id).await?;

        Ok(TargetItem {
            id: target_id,
            name: cat.name,
            category_type: to_v2_category_type(cat.category_type),
            parent_id: cat.parent_id,
            previous_target: None,
            current_target: Some(request.value),
            projected_variance: 0,
            status: TargetStatus::Active,
            spent_in_period: 0,
        })
    }

    pub async fn update_target(&self, target_id: &Uuid, request: &CreateTargetRequest, user_id: &Uuid) -> Result<TargetItem, AppError> {
        self.repository.update_target(target_id, request.value, user_id).await?;

        let target_row = self
            .repository
            .get_target_by_id(target_id, user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Target not found".to_string()))?;

        let cat = self
            .repository
            .get_category_by_id(&target_row.category_id, user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Category not found".to_string()))?;

        Ok(TargetItem {
            id: target_row.id,
            name: cat.name,
            category_type: to_v2_category_type(cat.category_type),
            parent_id: cat.parent_id,
            previous_target: None,
            current_target: Some(target_row.budgeted_value),
            projected_variance: 0,
            status: if target_row.is_excluded {
                TargetStatus::Excluded
            } else {
                TargetStatus::Active
            },
            spent_in_period: 0,
        })
    }

    pub async fn exclude_target(&self, target_id: &Uuid, user_id: &Uuid) -> Result<TargetItem, AppError> {
        let target_row = self
            .repository
            .get_target_by_id(target_id, user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Target not found".to_string()))?;

        if target_row.is_excluded {
            return Err(AppError::Conflict("Target is already excluded".to_string()));
        }

        self.repository.exclude_target(target_id, user_id).await?;

        let cat = self
            .repository
            .get_category_by_id(&target_row.category_id, user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Category not found".to_string()))?;

        Ok(TargetItem {
            id: target_row.id,
            name: cat.name,
            category_type: to_v2_category_type(cat.category_type),
            parent_id: cat.parent_id,
            previous_target: None,
            current_target: Some(target_row.budgeted_value),
            projected_variance: 0,
            status: TargetStatus::Excluded,
            spent_in_period: 0,
        })
    }
}
