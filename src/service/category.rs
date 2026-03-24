use crate::database::postgres_repository::PostgresRepository;
use crate::dto::categories::{
    CategoriesWithTargets, CategoryBase, CategoryDetailResponse, CategoryManagementListItem, CategoryManagementListResponse, CategoryOptionListResponse,
    CategoryOptionResponse, CategoryOverviewResponse, CategoryOverviewSummary, CategoryResponse, CategoryStabilityDot, CategorySummaryItem,
    CategoryTargetsResponse, CategoryTransactionItem, CategoryTrendItem, CategoryTrendResponse, CategoryType, CreateCategoryRequest, CreateTargetRequest,
    TargetItem, TargetStatus, TargetSummary, UpdateTargetRequest, compute_color,
};
use crate::dto::common::{Date, PaginatedResponse};
use crate::error::app_error::AppError;
use crate::models::category::{CategoryRequest, CategoryType as V1CategoryType};
use uuid::Uuid;

pub struct CategoryService<'a> {
    repository: &'a PostgresRepository,
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
                base: CategoryBase::from_model(cat),
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

    pub async fn create_category(&self, request: &CreateCategoryRequest, user_id: &Uuid) -> Result<CategoryResponse, AppError> {
        let v1_type = request.category_type.to_v1();
        let v1_behavior = request.behavior.map(|b| b.to_v1());
        let computed_color = compute_color(v1_type, v1_behavior);
        let v1_request = CategoryRequest {
            name: request.name.clone(),
            color: computed_color,
            icon: request.icon.clone(),
            parent_id: request.parent_id,
            category_type: v1_type,
            description: request.description.clone(),
            behavior: request
                .behavior
                .map(|b| crate::models::category::category_behavior_to_db(b.to_v1()).to_string()),
        };

        let category = self.repository.create_category(&v1_request, user_id).await?;
        Ok(CategoryResponse::from_model(&category))
    }

    pub async fn update_category(&self, id: &Uuid, request: &CreateCategoryRequest, user_id: &Uuid) -> Result<CategoryResponse, AppError> {
        let v1_type = request.category_type.to_v1();
        let v1_behavior = request.behavior.map(|b| b.to_v1());
        let computed_color = compute_color(v1_type, v1_behavior);
        let v1_request = CategoryRequest {
            name: request.name.clone(),
            color: computed_color,
            icon: request.icon.clone(),
            parent_id: request.parent_id,
            category_type: v1_type,
            description: request.description.clone(),
            behavior: request
                .behavior
                .map(|b| crate::models::category::category_behavior_to_db(b.to_v1()).to_string()),
        };

        let category = self.repository.update_category(id, &v1_request, user_id).await?;
        Ok(CategoryResponse::from_model(&category))
    }

    pub async fn delete_category(&self, id: &Uuid, user_id: &Uuid) -> Result<(), AppError> {
        self.repository
            .get_category_by_id(id, user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Category not found".to_string()))?;

        self.repository.delete_category(id, user_id).await
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
                color: compute_color(c.category_type, c.behavior),
            })
            .collect())
    }

    pub async fn get_category_overview(&self, period_id: &Uuid, user_id: &Uuid) -> Result<CategoryOverviewResponse, AppError> {
        let period = self.repository.get_budget_period(period_id, user_id).await?;

        let today = chrono::Utc::now().date_naive();
        let total_days = (period.end_date - period.start_date).num_days().max(1);
        let elapsed_days = (today - period.start_date).num_days().clamp(0, total_days);
        let period_elapsed_percent = (elapsed_days * 100) / total_days;

        let category_data = self
            .repository
            .get_category_overview_data(&period.start_date, &period.end_date, user_id)
            .await?;

        let mut total_spent: i64 = 0;
        let mut total_budgeted: Option<i64> = None;

        let categories: Vec<CategorySummaryItem> = category_data
            .iter()
            .map(|row| {
                let actual = row.actual;
                let budgeted = row.budgeted;

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
                    base: CategoryBase::from_model(&row.category),
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
        let cat = self
            .repository
            .get_category_by_id(id, user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Category not found".to_string()))?;

        if cat.is_archived {
            return Err(AppError::Conflict("Category is already archived".to_string()));
        }

        let archived = self.repository.archive_category(id, user_id).await?;
        Ok(CategoryResponse::from_model(&archived))
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
        Ok(CategoryResponse::from_model(&restored))
    }

    // ===== Targets =====

    pub async fn list_targets(&self, period_id: &Uuid, user_id: &Uuid) -> Result<CategoryTargetsResponse, AppError> {
        let period = self.repository.get_budget_period(period_id, user_id).await?;

        let today = chrono::Utc::now().date_naive();
        let total_days = (period.end_date - period.start_date).num_days().max(1);
        let elapsed_days = (today - period.start_date).num_days().clamp(0, total_days);
        let period_progress = (elapsed_days * 100) / total_days;

        let target_rows = self.repository.list_targets_v2(&period.start_date, &period.end_date, user_id).await?;

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
                    category_type: CategoryType::from_v1(row.category_type),
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
        let cat = self
            .repository
            .get_category_by_id(&request.category_id, user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Category not found".to_string()))?;

        let existing = self.repository.get_target_for_category(&request.category_id, user_id).await?;
        if existing.is_some() {
            return Err(AppError::Conflict("Target already exists for this category".to_string()));
        }

        let target_id = self.repository.create_target(&request.category_id, request.value, user_id).await?;

        Ok(TargetItem {
            id: target_id,
            name: cat.name,
            category_type: CategoryType::from_v1(cat.category_type),
            parent_id: cat.parent_id,
            previous_target: None,
            current_target: Some(request.value),
            projected_variance: 0,
            status: TargetStatus::Active,
            spent_in_period: 0,
        })
    }

    pub async fn update_target(&self, target_id: &Uuid, request: &UpdateTargetRequest, user_id: &Uuid) -> Result<TargetItem, AppError> {
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
            category_type: CategoryType::from_v1(cat.category_type),
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
            category_type: CategoryType::from_v1(cat.category_type),
            parent_id: cat.parent_id,
            previous_target: None,
            current_target: Some(target_row.budgeted_value),
            projected_variance: 0,
            status: TargetStatus::Excluded,
            spent_in_period: 0,
        })
    }

    pub async fn get_category_detail(&self, category_id: &Uuid, period_id: &Uuid, user_id: &Uuid) -> Result<CategoryDetailResponse, AppError> {
        let db = self
            .repository
            .get_category_detail_v2(category_id, user_id, period_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Category not found".to_string()))?;

        let variance = db.budgeted.map(|b| b - db.period_spend).unwrap_or(0);

        let stability_dots = db
            .stability_rows
            .into_iter()
            .map(|r| {
                let within_budget = r.budget.map(|b| r.spent <= b).unwrap_or(true);
                CategoryStabilityDot {
                    period_id: r.period_id,
                    period_name: r.period_name,
                    within_budget,
                }
            })
            .collect();

        let recent_transactions = db
            .recent_txns
            .into_iter()
            .map(|r| CategoryTransactionItem {
                id: r.id,
                date: Date(r.date),
                amount: r.amount,
                description: r.description,
                vendor_id: r.vendor_id,
                vendor_name: r.vendor_name,
            })
            .collect();

        Ok(CategoryDetailResponse {
            base: CategoryResponse::from_model(&db.category),
            period_spent: db.period_spend,

            budgeted: db.budgeted,
            variance,
            stability_dots,
            recent_transactions,
        })
    }

    pub async fn get_category_trend(&self, category_id: &Uuid, limit: i64, user_id: &Uuid) -> Result<CategoryTrendResponse, AppError> {
        let rows = self
            .repository
            .get_category_trend_v2(category_id, user_id, limit)
            .await?
            .ok_or_else(|| AppError::NotFound("Category not found".to_string()))?;

        Ok(rows
            .into_iter()
            .map(|r| CategoryTrendItem {
                period_id: r.period_id,
                period_name: r.period_name,
                total_spend: r.total_spend,
            })
            .collect())
    }
}
