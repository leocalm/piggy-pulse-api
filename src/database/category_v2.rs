use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::models::category::{Category, CategoryType};
use uuid::Uuid;

use super::category::category_type_from_db;

// ===== V2 Categories =====

#[derive(Debug, sqlx::FromRow)]
struct CategoryWithTxCountRow {
    id: Uuid,
    name: String,
    color: String,
    icon: String,
    parent_id: Option<Uuid>,
    category_type: String,
    is_archived: bool,
    description: Option<String>,
    is_system: bool,
    transaction_count: i64,
}

impl PostgresRepository {
    /// List categories for V2 management list with all-time transaction counts.
    /// Returns (Vec<(Category, transaction_count)>, total_count).
    /// Fetches limit+1 rows for sentinel-based pagination.
    pub async fn list_categories_v2(&self, cursor: Option<Uuid>, limit: i64, user_id: &Uuid) -> Result<(Vec<(Category, i64)>, i64), AppError> {
        let fetch_limit = limit + 1;

        let total_count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)::bigint
            FROM category
            WHERE user_id = $1 AND is_system = FALSE
            "#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        let rows = if let Some(cursor_id) = cursor {
            sqlx::query_as::<_, CategoryWithTxCountRow>(
                r#"
                WITH transaction_counts AS (
                    SELECT category_id, COUNT(*)::bigint AS transaction_count
                    FROM transaction
                    WHERE user_id = $1
                    GROUP BY category_id
                )
                SELECT
                    c.id, c.name,
                    COALESCE(c.color, '') as color,
                    COALESCE(c.icon, '') as icon,
                    c.parent_id,
                    c.category_type::text as category_type,
                    c.is_archived, c.description, c.is_system,
                    COALESCE(tc.transaction_count, 0) AS transaction_count
                FROM category c
                LEFT JOIN transaction_counts tc ON c.id = tc.category_id
                WHERE c.user_id = $1
                  AND c.is_system = FALSE
                  AND (c.created_at, c.id) < (SELECT created_at, id FROM category WHERE id = $2)
                ORDER BY c.created_at DESC, c.id DESC
                LIMIT $3
                "#,
            )
            .bind(user_id)
            .bind(cursor_id)
            .bind(fetch_limit)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, CategoryWithTxCountRow>(
                r#"
                WITH transaction_counts AS (
                    SELECT category_id, COUNT(*)::bigint AS transaction_count
                    FROM transaction
                    WHERE user_id = $1
                    GROUP BY category_id
                )
                SELECT
                    c.id, c.name,
                    COALESCE(c.color, '') as color,
                    COALESCE(c.icon, '') as icon,
                    c.parent_id,
                    c.category_type::text as category_type,
                    c.is_archived, c.description, c.is_system,
                    COALESCE(tc.transaction_count, 0) AS transaction_count
                FROM category c
                LEFT JOIN transaction_counts tc ON c.id = tc.category_id
                WHERE c.user_id = $1
                  AND c.is_system = FALSE
                ORDER BY c.created_at DESC, c.id DESC
                LIMIT $2
                "#,
            )
            .bind(user_id)
            .bind(fetch_limit)
            .fetch_all(&self.pool)
            .await?
        };

        let data = rows
            .into_iter()
            .map(|row| {
                let cat = Category {
                    id: row.id,
                    name: row.name,
                    color: row.color,
                    icon: row.icon,
                    parent_id: row.parent_id,
                    category_type: category_type_from_db(&row.category_type),
                    is_archived: row.is_archived,
                    description: row.description,
                    is_system: row.is_system,
                };
                (cat, row.transaction_count)
            })
            .collect();

        Ok((data, total_count))
    }

    // ===== V2 Category Overview =====

    pub async fn get_category_overview_data(
        &self,
        _period_id: &Uuid,
        start_date: &chrono::NaiveDate,
        end_date: &chrono::NaiveDate,
        user_id: &Uuid,
    ) -> Result<Vec<CategoryOverviewRow>, AppError> {
        #[derive(Debug, sqlx::FromRow)]
        struct OverviewDbRow {
            id: Uuid,
            name: String,
            color: String,
            icon: String,
            parent_id: Option<Uuid>,
            category_type: String,
            is_archived: bool,
            description: Option<String>,
            is_system: bool,
            actual: i64,
            budgeted: Option<i64>,
        }

        let rows = sqlx::query_as::<_, OverviewDbRow>(
            r#"
            WITH period_spend AS (
                SELECT
                    t.category_id,
                    COALESCE(SUM(t.amount), 0)::bigint AS actual
                FROM transaction t
                WHERE t.user_id = $1
                  AND t.occurred_at >= $2
                  AND t.occurred_at <= $3
                GROUP BY t.category_id
            )
            SELECT
                c.id, c.name,
                COALESCE(c.color, '') as color,
                COALESCE(c.icon, '') as icon,
                c.parent_id,
                c.category_type::text as category_type,
                c.is_archived, c.description, c.is_system,
                COALESCE(ps.actual, 0) AS actual,
                CASE WHEN bc.budgeted_value IS NOT NULL AND bc.is_excluded = FALSE
                     THEN bc.budgeted_value::bigint
                     ELSE NULL
                END AS budgeted
            FROM category c
            LEFT JOIN period_spend ps ON c.id = ps.category_id
            LEFT JOIN budget_category bc ON bc.category_id = c.id AND bc.user_id = $1
            WHERE c.user_id = $1
              AND c.is_system = FALSE
              AND c.is_archived = FALSE
            ORDER BY c.name ASC
            "#,
        )
        .bind(user_id)
        .bind(start_date)
        .bind(end_date)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| CategoryOverviewRow {
                category: Category {
                    id: row.id,
                    name: row.name,
                    color: row.color,
                    icon: row.icon,
                    parent_id: row.parent_id,
                    category_type: category_type_from_db(&row.category_type),
                    is_archived: row.is_archived,
                    description: row.description,
                    is_system: row.is_system,
                },
                actual: row.actual,
                budgeted: row.budgeted,
            })
            .collect())
    }

    // ===== V2 Targets =====

    pub async fn list_targets_v2(
        &self,
        _period_id: &Uuid,
        start_date: &chrono::NaiveDate,
        end_date: &chrono::NaiveDate,
        user_id: &Uuid,
    ) -> Result<Vec<TargetListRow>, AppError> {
        #[derive(Debug, sqlx::FromRow)]
        struct RawRow {
            target_id: Uuid,
            category_id: Uuid,
            category_name: String,
            category_type: String,
            parent_id: Option<Uuid>,
            current_target: Option<i64>,
            is_excluded: bool,
            spent_in_period: i64,
        }

        // Note: budget_category has no period dimension, so previousTarget is
        // not available from this table. The field is always NULL until a
        // target-history mechanism is added.
        let rows = sqlx::query_as::<_, RawRow>(
            r#"
            WITH period_spend AS (
                SELECT
                    t.category_id,
                    COALESCE(SUM(t.amount), 0)::bigint AS spent
                FROM transaction t
                WHERE t.user_id = $1
                  AND t.occurred_at >= $2
                  AND t.occurred_at <= $3
                GROUP BY t.category_id
            )
            SELECT
                bc.id AS target_id,
                bc.category_id,
                c.name AS category_name,
                c.category_type::text AS category_type,
                c.parent_id,
                CASE WHEN bc.is_excluded = FALSE THEN bc.budgeted_value::bigint ELSE NULL END AS current_target,
                bc.is_excluded,
                COALESCE(ps.spent, 0) AS spent_in_period
            FROM budget_category bc
            JOIN category c ON c.id = bc.category_id AND c.user_id = $1
            LEFT JOIN period_spend ps ON ps.category_id = bc.category_id
            WHERE bc.user_id = $1
              AND c.category_type != 'Transfer'
            ORDER BY c.name ASC
            "#,
        )
        .bind(user_id)
        .bind(start_date)
        .bind(end_date)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| TargetListRow {
                target_id: row.target_id,
                category_id: row.category_id,
                category_name: row.category_name,
                category_type: category_type_from_db(&row.category_type),
                parent_id: row.parent_id,
                current_target: row.current_target,
                previous_target: None,
                is_excluded: row.is_excluded,
                spent_in_period: row.spent_in_period,
            })
            .collect())
    }

    /// Get target (budget_category) for a specific category
    pub async fn get_target_for_category(&self, category_id: &Uuid, user_id: &Uuid) -> Result<Option<BudgetCategoryRow>, AppError> {
        let row = sqlx::query_as::<_, BudgetCategoryDbRow>(
            r#"
            SELECT id, category_id, budgeted_value::bigint AS budgeted_value, is_excluded
            FROM budget_category
            WHERE category_id = $1 AND user_id = $2
            "#,
        )
        .bind(category_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| BudgetCategoryRow {
            id: r.id,
            category_id: r.category_id,
            budgeted_value: r.budgeted_value,
            is_excluded: r.is_excluded,
        }))
    }

    /// Create a new target (budget_category row)
    pub async fn create_target(&self, category_id: &Uuid, value: i64, user_id: &Uuid) -> Result<Uuid, AppError> {
        let id: Uuid = sqlx::query_scalar(
            r#"
            INSERT INTO budget_category (user_id, category_id, budgeted_value, is_excluded)
            VALUES ($1, $2, $3, FALSE)
            RETURNING id
            "#,
        )
        .bind(user_id)
        .bind(category_id)
        .bind(value as i32)
        .fetch_one(&self.pool)
        .await?;

        Ok(id)
    }

    /// Update a target's budgeted value
    pub async fn update_target(&self, target_id: &Uuid, value: i64, user_id: &Uuid) -> Result<(), AppError> {
        let rows_affected = sqlx::query(
            r#"
            UPDATE budget_category
            SET budgeted_value = $1
            WHERE id = $2 AND user_id = $3
            "#,
        )
        .bind(value as i32)
        .bind(target_id)
        .bind(user_id)
        .execute(&self.pool)
        .await?
        .rows_affected();

        if rows_affected == 0 {
            return Err(AppError::NotFound("Target not found".to_string()));
        }

        Ok(())
    }

    /// Get a target by its ID
    pub async fn get_target_by_id(&self, target_id: &Uuid, user_id: &Uuid) -> Result<Option<BudgetCategoryRow>, AppError> {
        let row = sqlx::query_as::<_, BudgetCategoryDbRow>(
            r#"
            SELECT id, category_id, budgeted_value::bigint AS budgeted_value, is_excluded
            FROM budget_category
            WHERE id = $1 AND user_id = $2
            "#,
        )
        .bind(target_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| BudgetCategoryRow {
            id: r.id,
            category_id: r.category_id,
            budgeted_value: r.budgeted_value,
            is_excluded: r.is_excluded,
        }))
    }

    /// Exclude a target
    pub async fn exclude_target(&self, target_id: &Uuid, user_id: &Uuid) -> Result<(), AppError> {
        let rows_affected = sqlx::query(
            r#"
            UPDATE budget_category
            SET is_excluded = TRUE
            WHERE id = $1 AND user_id = $2
            "#,
        )
        .bind(target_id)
        .bind(user_id)
        .execute(&self.pool)
        .await?
        .rows_affected();

        if rows_affected == 0 {
            return Err(AppError::NotFound("Target not found".to_string()));
        }

        Ok(())
    }
}

// ===== Row types =====

pub struct CategoryOverviewRow {
    pub category: Category,
    pub actual: i64,
    pub budgeted: Option<i64>,
}

#[allow(dead_code)]
pub struct TargetListRow {
    pub target_id: Uuid,
    pub category_id: Uuid,
    pub category_name: String,
    pub category_type: CategoryType,
    pub parent_id: Option<Uuid>,
    pub current_target: Option<i64>,
    pub previous_target: Option<i64>,
    pub is_excluded: bool,
    pub spent_in_period: i64,
}

#[derive(Debug, sqlx::FromRow)]
struct BudgetCategoryDbRow {
    id: Uuid,
    category_id: Uuid,
    budgeted_value: i64,
    is_excluded: bool,
}

pub struct BudgetCategoryRow {
    pub id: Uuid,
    pub category_id: Uuid,
    pub budgeted_value: i64,
    pub is_excluded: bool,
}
