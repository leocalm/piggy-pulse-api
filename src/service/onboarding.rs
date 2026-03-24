use crate::database::postgres_repository::PostgresRepository;
use crate::dto::categories::CategoryResponse;
use crate::dto::misc::{ApplyTemplateRequest, CategoryTemplateCategory, CategoryTemplateResponse, OnboardingStatus, OnboardingStatusResponse, OnboardingStep};
use crate::error::app_error::AppError;
use crate::models::category::CategoryRequest;
use uuid::Uuid;

// ── Static template definitions ───────────────────────────────────────────────

struct TemplateDef {
    id: &'static str,
    name: &'static str,
    description: &'static str,
    categories: &'static [CategoryDef],
}

struct CategoryDef {
    name: &'static str,
    category_type: &'static str,
    behavior: Option<&'static str>,
    icon: &'static str,
}

const TEMPLATES: &[TemplateDef] = &[
    TemplateDef {
        id: "essential",
        name: "Essential 5",
        description: "Five fundamental categories to get started with budgeting.",
        categories: &[
            CategoryDef {
                name: "Salary",
                category_type: "income",
                behavior: Some("variable"),
                icon: "💰",
            },
            CategoryDef {
                name: "Groceries",
                category_type: "expense",
                behavior: Some("variable"),
                icon: "🛒",
            },
            CategoryDef {
                name: "Rent & Housing",
                category_type: "expense",
                behavior: Some("fixed"),
                icon: "🏠",
            },
            CategoryDef {
                name: "Transport",
                category_type: "expense",
                behavior: Some("variable"),
                icon: "🚗",
            },
            CategoryDef {
                name: "Entertainment",
                category_type: "expense",
                behavior: Some("variable"),
                icon: "🎬",
            },
        ],
    },
    TemplateDef {
        id: "detailed",
        name: "Detailed 12",
        description: "A comprehensive set of twelve categories covering most personal finance needs.",
        categories: &[
            CategoryDef {
                name: "Salary",
                category_type: "income",
                behavior: Some("fixed"),
                icon: "💰",
            },
            CategoryDef {
                name: "Freelance",
                category_type: "income",
                behavior: Some("variable"),
                icon: "💼",
            },
            CategoryDef {
                name: "Rent & Housing",
                category_type: "expense",
                behavior: Some("fixed"),
                icon: "🏠",
            },
            CategoryDef {
                name: "Utilities",
                category_type: "expense",
                behavior: Some("fixed"),
                icon: "⚡",
            },
            CategoryDef {
                name: "Groceries",
                category_type: "expense",
                behavior: Some("variable"),
                icon: "🛒",
            },
            CategoryDef {
                name: "Dining Out",
                category_type: "expense",
                behavior: Some("variable"),
                icon: "🍽️",
            },
            CategoryDef {
                name: "Transport",
                category_type: "expense",
                behavior: Some("variable"),
                icon: "🚗",
            },
            CategoryDef {
                name: "Healthcare",
                category_type: "expense",
                behavior: Some("variable"),
                icon: "🏥",
            },
            CategoryDef {
                name: "Entertainment",
                category_type: "expense",
                behavior: Some("variable"),
                icon: "🎬",
            },
            CategoryDef {
                name: "Shopping",
                category_type: "expense",
                behavior: Some("variable"),
                icon: "🛍️",
            },
            CategoryDef {
                name: "Subscriptions",
                category_type: "expense",
                behavior: Some("subscription"),
                icon: "📱",
            },
            CategoryDef {
                name: "Education",
                category_type: "expense",
                behavior: Some("variable"),
                icon: "📚",
            },
        ],
    },
];

pub struct OnboardingService<'a> {
    repository: &'a PostgresRepository,
}

impl<'a> OnboardingService<'a> {
    pub fn new(repository: &'a PostgresRepository) -> Self {
        OnboardingService { repository }
    }

    pub async fn get_status(&self, user_id: &Uuid) -> Result<OnboardingStatusResponse, AppError> {
        let onboarding_status: String = sqlx::query_scalar("SELECT onboarding_status FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_one(&self.repository.pool)
            .await
            .map_err(AppError::from)?;

        if onboarding_status == "completed" {
            return Ok(OnboardingStatusResponse {
                status: OnboardingStatus::Completed,
                current_step: None,
            });
        }

        let current_step = self.derive_current_step(user_id).await?;

        let status = if matches!(current_step, Some(OnboardingStep::Currency)) {
            OnboardingStatus::NotStarted
        } else {
            OnboardingStatus::InProgress
        };

        Ok(OnboardingStatusResponse { status, current_step })
    }

    pub async fn complete(&self, user_id: &Uuid) -> Result<(), AppError> {
        let onboarding_status: String = sqlx::query_scalar("SELECT onboarding_status FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_one(&self.repository.pool)
            .await
            .map_err(AppError::from)?;

        if onboarding_status == "completed" {
            self.repository.generate_automatic_budget_periods().await?;
            return Ok(());
        }

        let current_step = self.derive_current_step(user_id).await?;
        if !matches!(current_step, Some(OnboardingStep::Summary)) {
            return Err(AppError::BadRequest("Onboarding steps are not yet complete".to_string()));
        }

        sqlx::query("UPDATE users SET onboarding_status = 'completed' WHERE id = $1")
            .bind(user_id)
            .execute(&self.repository.pool)
            .await
            .map_err(AppError::from)?;

        self.repository.generate_automatic_budget_periods().await?;

        Ok(())
    }

    pub fn list_templates(&self) -> Vec<CategoryTemplateResponse> {
        TEMPLATES
            .iter()
            .map(|t| CategoryTemplateResponse {
                id: t.id.to_string(),
                name: t.name.to_string(),
                description: t.description.to_string(),
                categories: t
                    .categories
                    .iter()
                    .map(|c| CategoryTemplateCategory {
                        name: c.name.to_string(),
                        category_type: c.category_type.to_string(),
                        behavior: c.behavior.map(|b| b.to_string()),
                        icon: c.icon.to_string(),
                    })
                    .collect(),
            })
            .collect()
    }

    pub async fn apply_template(&self, request: &ApplyTemplateRequest, user_id: &Uuid) -> Result<Vec<CategoryResponse>, AppError> {
        use crate::dto::categories::{CategoryBehavior, CategoryType};

        let template = TEMPLATES
            .iter()
            .find(|t| t.id == request.template_id)
            .ok_or_else(|| AppError::NotFound(format!("Template '{}' not found", request.template_id)))?;

        let mut created = Vec::with_capacity(template.categories.len());

        for cat_def in template.categories {
            let v2_type: CategoryType = match cat_def.category_type {
                "income" => CategoryType::Income,
                "expense" => CategoryType::Expense,
                _ => CategoryType::Transfer,
            };
            let v2_behavior: Option<CategoryBehavior> = cat_def.behavior.and_then(|b| match b {
                "fixed" => Some(CategoryBehavior::Fixed),
                "variable" => Some(CategoryBehavior::Variable),
                "subscription" => Some(CategoryBehavior::Subscription),
                _ => None,
            });

            let v1_type = v2_type.to_v1();
            let v1_behavior = v2_behavior.map(|b| b.to_v1());
            let computed_color = crate::dto::categories::compute_color(v1_type, v1_behavior);

            let v1_request = CategoryRequest {
                name: cat_def.name.to_string(),
                color: computed_color,
                icon: cat_def.icon.to_string(),
                parent_id: None,
                category_type: v1_type,
                description: None,
                behavior: v1_behavior.map(|b| crate::models::category::category_behavior_to_db(b).to_string()),
            };

            let category = self.repository.create_category(&v1_request, user_id).await?;
            created.push(CategoryResponse::from_model(&category));
        }

        Ok(created)
    }

    async fn derive_current_step(&self, user_id: &Uuid) -> Result<Option<OnboardingStep>, AppError> {
        let has_currency: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM settings WHERE user_id = $1 AND default_currency_id IS NOT NULL)")
            .bind(user_id)
            .fetch_one(&self.repository.pool)
            .await
            .map_err(AppError::from)?;

        if !has_currency {
            return Ok(Some(OnboardingStep::Currency));
        }

        let has_period: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM period_schedule WHERE user_id = $1)")
            .bind(user_id)
            .fetch_one(&self.repository.pool)
            .await
            .map_err(AppError::from)?;

        if !has_period {
            return Ok(Some(OnboardingStep::Period));
        }

        let has_accounts: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM account WHERE user_id = $1 AND is_archived = FALSE)")
            .bind(user_id)
            .fetch_one(&self.repository.pool)
            .await
            .map_err(AppError::from)?;

        if !has_accounts {
            return Ok(Some(OnboardingStep::Accounts));
        }

        let has_incoming: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM category WHERE user_id = $1 AND is_archived = FALSE AND is_system = FALSE AND category_type = 'Incoming'::category_type)",
        )
        .bind(user_id)
        .fetch_one(&self.repository.pool)
        .await
        .map_err(AppError::from)?;

        let has_outgoing: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM category WHERE user_id = $1 AND is_archived = FALSE AND is_system = FALSE AND category_type = 'Outgoing'::category_type)",
        )
        .bind(user_id)
        .fetch_one(&self.repository.pool)
        .await
        .map_err(AppError::from)?;

        if !has_incoming || !has_outgoing {
            return Ok(Some(OnboardingStep::Categories));
        }

        Ok(Some(OnboardingStep::Summary))
    }
}
