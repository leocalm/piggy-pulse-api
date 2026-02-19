use crate::database::postgres_repository::{PostgresRepository, is_unique_violation};
use crate::error::app_error::AppError;
use crate::models::budget_period::BudgetPeriod;
use crate::models::category::{
    Category, CategoryBudgetedDiagnosticsRow, CategoryRequest, CategoryStats, CategoryType, CategoryUnbudgetedDiagnosticsRow, CategoryWithStats,
    difference_vs_average_percentage, progress_basis_points, share_of_total_basis_points, variance_value,
};
use crate::models::dashboard::BudgetStabilityPeriodResponse;
use crate::models::pagination::CursorParams;
use crate::service::dashboard::is_outside_tolerance;
use std::collections::HashMap;
use uuid::Uuid;

// Intermediate struct for sqlx query results with category_type as text
#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct CategoryRow {
    id: Uuid,
    name: String,
    color: String,
    icon: String,
    parent_id: Option<Uuid>,
    category_type: String,
}

impl From<CategoryRow> for Category {
    fn from(row: CategoryRow) -> Self {
        Category {
            id: row.id,
            name: row.name,
            color: row.color,
            icon: row.icon,
            parent_id: row.parent_id,
            category_type: category_type_from_db(&row.category_type),
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct CategoryWithStatsRow {
    id: Uuid,
    name: String,
    color: String,
    icon: String,
    parent_id: Option<Uuid>,
    category_type: String,
    used_in_period: i64,
    average_period_usage: i64,
    transaction_count: i64,
}

#[derive(Debug, sqlx::FromRow)]
struct BudgetedCategoryDiagnosticsDbRow {
    id: Uuid,
    name: String,
    color: String,
    icon: String,
    parent_id: Option<Uuid>,
    category_type: String,
    budgeted_value: i32,
    actual_value: i64,
}

#[derive(Debug, sqlx::FromRow)]
struct UnbudgetedCategoryDiagnosticsDbRow {
    id: Uuid,
    name: String,
    color: String,
    icon: String,
    parent_id: Option<Uuid>,
    category_type: String,
    actual_value: i64,
}

#[derive(Debug, sqlx::FromRow)]
struct BudgetedCategoryClosedPeriodDbRow {
    category_id: Uuid,
    period_id: Uuid,
    actual_value: i64,
}

impl From<CategoryWithStatsRow> for CategoryWithStats {
    fn from(row: CategoryWithStatsRow) -> Self {
        CategoryWithStats {
            category: Category {
                id: row.id,
                name: row.name,
                color: row.color,
                icon: row.icon,
                parent_id: row.parent_id,
                category_type: category_type_from_db(&row.category_type),
            },
            stats: CategoryStats {
                used_in_period: row.used_in_period,
                difference_vs_average_percentage: difference_vs_average_percentage(row.used_in_period, row.average_period_usage),
                transaction_count: row.transaction_count,
            },
        }
    }
}

impl From<BudgetedCategoryDiagnosticsDbRow> for CategoryBudgetedDiagnosticsRow {
    fn from(row: BudgetedCategoryDiagnosticsDbRow) -> Self {
        Self {
            category: Category {
                id: row.id,
                name: row.name,
                color: row.color,
                icon: row.icon,
                parent_id: row.parent_id,
                category_type: category_type_from_db(&row.category_type),
            },
            budgeted_value: row.budgeted_value,
            actual_value: row.actual_value,
            variance_value: variance_value(row.actual_value, row.budgeted_value),
            progress_basis_points: progress_basis_points(row.actual_value, row.budgeted_value),
            recent_closed_periods: Vec::new(),
        }
    }
}

impl PostgresRepository {
    pub async fn create_category(&self, request: &CategoryRequest, user_id: &Uuid) -> Result<Category, AppError> {
        let name_exists: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS (
                SELECT 1
                FROM category
                WHERE user_id = $1 AND name = $2
            )
            "#,
        )
        .bind(user_id)
        .bind(&request.name)
        .fetch_one(&self.pool)
        .await?;

        if name_exists {
            return Err(AppError::BadRequest("Category name already exists".to_string()));
        }

        let row = sqlx::query_as::<_, CategoryRow>(
            r#"
            INSERT INTO category (user_id, name, color, icon, parent_id, category_type)
            VALUES ($1, $2, $3, $4, $5, $6::text::category_type)
            RETURNING
                id,
                name,
                COALESCE(color, '') as color,
                COALESCE(icon, '') as icon,
                parent_id,
                category_type::text as category_type
            "#,
        )
        .bind(user_id)
        .bind(&request.name)
        .bind(&request.color)
        .bind(&request.icon)
        .bind(request.parent_id)
        .bind(request.category_type_to_db())
        .fetch_one(&self.pool)
        .await;

        let row = match row {
            Ok(row) => row,
            Err(err) if is_unique_violation(&err) => {
                return Err(AppError::BadRequest("Category name already exists".to_string()));
            }
            Err(err) => return Err(err.into()),
        };

        Ok(Category::from(row))
    }

    pub async fn get_category_by_id(&self, id: &Uuid, user_id: &Uuid) -> Result<Option<Category>, AppError> {
        let row = sqlx::query_as::<_, CategoryRow>(
            r#"
            SELECT
                id,
                name,
                COALESCE(color, '') as color,
                COALESCE(icon, '') as icon,
                parent_id,
                category_type::text as category_type
            FROM category
            WHERE id = $1 AND user_id = $2
            "#,
        )
        .bind(id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Category::from))
    }

    pub async fn list_categories(&self, params: &CursorParams, user_id: &Uuid, period: &BudgetPeriod) -> Result<Vec<CategoryWithStats>, AppError> {
        let rows = if let Some(cursor) = params.cursor {
            sqlx::query_as::<_, CategoryWithStatsRow>(
                r#"
WITH selected_period AS (
    SELECT $2::date AS start_date, $3::date AS end_date
),
period_totals AS (
    SELECT
        bp.id AS period_id,
        t.category_id,
        SUM(t.amount)::bigint AS period_amount
    FROM transaction t
    JOIN budget_period bp
        ON t.occurred_at >= bp.start_date
       AND t.occurred_at <= bp.end_date
       AND bp.user_id = $1
    WHERE t.user_id = $1
    GROUP BY bp.id, t.category_id
),
average_totals AS (
    SELECT
        category_id,
        COALESCE(AVG(period_amount), 0)::bigint AS avg_period_amount
    FROM period_totals
    GROUP BY category_id
),
selected_period_totals AS (
    SELECT
        t.category_id,
        COALESCE(SUM(t.amount), 0)::bigint AS used_this_period
    FROM transaction t
    CROSS JOIN selected_period sp
    WHERE t.user_id = $1
      AND t.occurred_at >= sp.start_date
      AND t.occurred_at <= sp.end_date
    GROUP BY t.category_id
),
selected_period_counts AS (
    SELECT
        t.category_id,
        COUNT(*)::bigint AS transaction_count
    FROM transaction t
    CROSS JOIN selected_period sp
    WHERE t.user_id = $1
      AND t.occurred_at >= sp.start_date
      AND t.occurred_at <= sp.end_date
    GROUP BY t.category_id
)
SELECT
    c.id,
    c.name,
    COALESCE(c.color, '') as color,
    COALESCE(c.icon, '') as icon,
    c.parent_id,
    c.category_type::text as category_type,
    COALESCE(spt.used_this_period, 0) AS used_in_period,
    COALESCE(at.avg_period_amount, 0) AS average_period_usage,
    COALESCE(spc.transaction_count, 0) AS transaction_count
FROM category c
LEFT JOIN selected_period_totals spt ON c.id = spt.category_id
LEFT JOIN average_totals at ON c.id = at.category_id
LEFT JOIN selected_period_counts spc ON c.id = spc.category_id
WHERE c.user_id = $1
  AND (c.created_at, c.id) < (SELECT created_at, id FROM category WHERE id = $4)
ORDER BY c.created_at DESC, c.id DESC
LIMIT $5
                "#,
            )
            .bind(user_id)
            .bind(period.start_date)
            .bind(period.end_date)
            .bind(cursor)
            .bind(params.fetch_limit())
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, CategoryWithStatsRow>(
                r#"
WITH selected_period AS (
    SELECT $2::date AS start_date, $3::date AS end_date
),
period_totals AS (
    SELECT
        bp.id AS period_id,
        t.category_id,
        SUM(t.amount)::bigint AS period_amount
    FROM transaction t
    JOIN budget_period bp
        ON t.occurred_at >= bp.start_date
       AND t.occurred_at <= bp.end_date
       AND bp.user_id = $1
    WHERE t.user_id = $1
    GROUP BY bp.id, t.category_id
),
average_totals AS (
    SELECT
        category_id,
        COALESCE(AVG(period_amount), 0)::bigint AS avg_period_amount
    FROM period_totals
    GROUP BY category_id
),
selected_period_totals AS (
    SELECT
        t.category_id,
        COALESCE(SUM(t.amount), 0)::bigint AS used_this_period
    FROM transaction t
    CROSS JOIN selected_period sp
    WHERE t.user_id = $1
      AND t.occurred_at >= sp.start_date
      AND t.occurred_at <= sp.end_date
    GROUP BY t.category_id
),
selected_period_counts AS (
    SELECT
        t.category_id,
        COUNT(*)::bigint AS transaction_count
    FROM transaction t
    CROSS JOIN selected_period sp
    WHERE t.user_id = $1
      AND t.occurred_at >= sp.start_date
      AND t.occurred_at <= sp.end_date
    GROUP BY t.category_id
)
SELECT
    c.id,
    c.name,
    COALESCE(c.color, '') as color,
    COALESCE(c.icon, '') as icon,
    c.parent_id,
    c.category_type::text as category_type,
    COALESCE(spt.used_this_period, 0) AS used_in_period,
    COALESCE(at.avg_period_amount, 0) AS average_period_usage,
    COALESCE(spc.transaction_count, 0) AS transaction_count
FROM category c
LEFT JOIN selected_period_totals spt ON c.id = spt.category_id
LEFT JOIN average_totals at ON c.id = at.category_id
LEFT JOIN selected_period_counts spc ON c.id = spc.category_id
WHERE c.user_id = $1
ORDER BY c.created_at DESC, c.id DESC
LIMIT $4
                "#,
            )
            .bind(user_id)
            .bind(period.start_date)
            .bind(period.end_date)
            .bind(params.fetch_limit())
            .fetch_all(&self.pool)
            .await?
        };

        Ok(rows.into_iter().map(CategoryWithStats::from).collect())
    }

    pub async fn list_budgeted_category_diagnostics(&self, user_id: &Uuid, period: &BudgetPeriod) -> Result<Vec<CategoryBudgetedDiagnosticsRow>, AppError> {
        let rows = sqlx::query_as::<_, BudgetedCategoryDiagnosticsDbRow>(
            r#"
WITH selected_period AS (
    SELECT $2::date AS start_date, $3::date AS end_date
),
selected_period_spend AS (
    SELECT
        t.category_id,
        COALESCE(SUM(t.amount), 0)::bigint AS actual_value
    FROM transaction t
    CROSS JOIN selected_period sp
    WHERE t.user_id = $1
      AND t.occurred_at >= sp.start_date
      AND t.occurred_at <= sp.end_date
    GROUP BY t.category_id
)
SELECT
    c.id,
    c.name,
    COALESCE(c.color, '') as color,
    COALESCE(c.icon, '') as icon,
    c.parent_id,
    c.category_type::text as category_type,
    bc.budgeted_value,
    COALESCE(sps.actual_value, 0) AS actual_value
FROM budget_category bc
JOIN category c
  ON c.id = bc.category_id
LEFT JOIN selected_period_spend sps
  ON sps.category_id = bc.category_id
WHERE bc.user_id = $1
  AND c.user_id = $1
  AND c.category_type = 'Outgoing'
ORDER BY bc.created_at DESC, bc.id DESC
            "#,
        )
        .bind(user_id)
        .bind(period.start_date)
        .bind(period.end_date)
        .fetch_all(&self.pool)
        .await?;

        let mut diagnostics: Vec<CategoryBudgetedDiagnosticsRow> = rows.into_iter().map(CategoryBudgetedDiagnosticsRow::from).collect();
        if diagnostics.is_empty() {
            return Ok(diagnostics);
        }

        let tolerance_basis_points = sqlx::query_scalar::<_, i32>(
            r#"
            SELECT budget_stability_tolerance_basis_points
            FROM settings
            WHERE user_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?
        .unwrap_or(1000);

        let closed_period_rows = sqlx::query_as::<_, BudgetedCategoryClosedPeriodDbRow>(
            r#"
WITH recent_closed_periods AS (
    SELECT
        bp.id,
        bp.start_date,
        bp.end_date
    FROM budget_period bp
    WHERE bp.user_id = $1
      AND bp.end_date < CURRENT_DATE
    ORDER BY bp.end_date DESC
    LIMIT 3
)
SELECT
    bc.category_id,
    rcp.id AS period_id,
    COALESCE(SUM(t.amount), 0)::bigint AS actual_value
FROM budget_category bc
JOIN category c
  ON c.id = bc.category_id
 AND c.user_id = $1
 AND c.category_type = 'Outgoing'
CROSS JOIN recent_closed_periods rcp
LEFT JOIN transaction t
  ON t.user_id = $1
 AND t.category_id = bc.category_id
 AND t.occurred_at >= rcp.start_date
 AND t.occurred_at <= rcp.end_date
WHERE bc.user_id = $1
GROUP BY bc.category_id, rcp.id, rcp.end_date
ORDER BY bc.category_id, rcp.end_date DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        let budget_by_category: HashMap<Uuid, i32> = diagnostics.iter().map(|row| (row.category.id, row.budgeted_value)).collect();
        let mut stability_by_category: HashMap<Uuid, Vec<BudgetStabilityPeriodResponse>> = HashMap::new();
        for row in closed_period_rows {
            let budgeted_value = i64::from(*budget_by_category.get(&row.category_id).unwrap_or(&0));
            stability_by_category.entry(row.category_id).or_default().push(BudgetStabilityPeriodResponse {
                period_id: row.period_id.to_string(),
                is_outside_tolerance: is_outside_tolerance(row.actual_value, budgeted_value, tolerance_basis_points),
            });
        }

        for periods in stability_by_category.values_mut() {
            periods.reverse();
        }

        for row in &mut diagnostics {
            row.recent_closed_periods = stability_by_category.remove(&row.category.id).unwrap_or_default();
        }

        Ok(diagnostics)
    }

    pub async fn list_unbudgeted_category_diagnostics(&self, user_id: &Uuid, period: &BudgetPeriod) -> Result<Vec<CategoryUnbudgetedDiagnosticsRow>, AppError> {
        let rows = sqlx::query_as::<_, UnbudgetedCategoryDiagnosticsDbRow>(
            r#"
WITH selected_period AS (
    SELECT $2::date AS start_date, $3::date AS end_date
)
SELECT
    c.id,
    c.name,
    COALESCE(c.color, '') as color,
    COALESCE(c.icon, '') as icon,
    c.parent_id,
    c.category_type::text as category_type,
    COALESCE(SUM(t.amount), 0)::bigint AS actual_value
FROM category c
         LEFT JOIN budget_category bc
                   ON bc.category_id = c.id
                       AND bc.user_id = $1
         LEFT JOIN transaction t
                   ON t.user_id = $1
                       AND t.category_id = c.id
                       AND t.occurred_at >= (SELECT start_date FROM selected_period)
                       AND t.occurred_at <= (SELECT end_date FROM selected_period)
WHERE c.user_id = $1
  AND bc.id IS NULL
GROUP BY c.id, c.name, c.color, c.icon, c.parent_id, c.category_type
ORDER BY actual_value DESC, c.name
            "#,
        )
        .bind(user_id)
        .bind(period.start_date)
        .bind(period.end_date)
        .fetch_all(&self.pool)
        .await?;

        let total_unbudgeted_actual = rows.iter().map(|row| row.actual_value.max(0)).sum::<i64>();

        Ok(rows
            .into_iter()
            .map(|row| CategoryUnbudgetedDiagnosticsRow {
                category: Category {
                    id: row.id,
                    name: row.name,
                    color: row.color,
                    icon: row.icon,
                    parent_id: row.parent_id,
                    category_type: category_type_from_db(&row.category_type),
                },
                actual_value: row.actual_value,
                share_of_total_basis_points: share_of_total_basis_points(row.actual_value, total_unbudgeted_actual),
            })
            .collect())
    }

    pub async fn delete_category(&self, id: &Uuid, user_id: &Uuid) -> Result<(), AppError> {
        sqlx::query("DELETE FROM category WHERE id = $1 AND user_id = $2")
            .bind(id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn update_category(&self, id: &Uuid, request: &CategoryRequest, user_id: &Uuid) -> Result<Category, AppError> {
        let name_exists: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS (
                SELECT 1
                FROM category
                WHERE user_id = $1 AND name = $2 AND id <> $3
            )
            "#,
        )
        .bind(user_id)
        .bind(&request.name)
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        if name_exists {
            return Err(AppError::BadRequest("Category name already exists".to_string()));
        }

        let row = sqlx::query_as::<_, CategoryRow>(
            r#"
            UPDATE category
            SET name = $1, color = $2, icon = $3, parent_id = $4, category_type = $5::text::category_type
            WHERE id = $6 AND user_id = $7
            RETURNING
                id,
                name,
                COALESCE(color, '') as color,
                COALESCE(icon, '') as icon,
                parent_id,
                category_type::text as category_type
            "#,
        )
        .bind(&request.name)
        .bind(&request.color)
        .bind(&request.icon)
        .bind(request.parent_id)
        .bind(request.category_type_to_db())
        .bind(id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await;

        let row = match row {
            Ok(row) => row,
            Err(err) if is_unique_violation(&err) => {
                return Err(AppError::BadRequest("Category name already exists".to_string()));
            }
            Err(err) => return Err(err.into()),
        };

        Ok(Category::from(row))
    }

    pub async fn list_categories_not_in_budget(&self, params: &CursorParams, user_id: &Uuid) -> Result<Vec<Category>, AppError> {
        let rows = if let Some(cursor) = params.cursor {
            sqlx::query_as::<_, CategoryRow>(
                r#"
                SELECT
                    c.id,
                    c.name,
                    COALESCE(c.color, '') as color,
                    COALESCE(c.icon, '') as icon,
                    c.parent_id,
                    c.category_type::text as category_type
                FROM category c
                LEFT JOIN budget_category bc ON c.id = bc.category_id
                WHERE bc.id IS NULL
                    AND c.category_type = 'Outgoing'
                    AND c.user_id = $1
                    AND (c.created_at, c.id) < (SELECT created_at, id FROM category WHERE id = $2)
                ORDER BY c.created_at DESC, c.id DESC
                LIMIT $3
                "#,
            )
            .bind(user_id)
            .bind(cursor)
            .bind(params.fetch_limit())
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, CategoryRow>(
                r#"
                SELECT
                    c.id,
                    c.name,
                    COALESCE(c.color, '') as color,
                    COALESCE(c.icon, '') as icon,
                    c.parent_id,
                    c.category_type::text as category_type
                FROM category c
                LEFT JOIN budget_category bc ON c.id = bc.category_id
                WHERE bc.id IS NULL
                    AND c.category_type = 'Outgoing'
                    AND c.user_id = $1
                ORDER BY c.created_at DESC, c.id DESC
                LIMIT $2
                "#,
            )
            .bind(user_id)
            .bind(params.fetch_limit())
            .fetch_all(&self.pool)
            .await?
        };

        Ok(rows.into_iter().map(Category::from).collect())
    }

    pub async fn list_all_categories(&self, user_id: &Uuid) -> Result<Vec<Category>, AppError> {
        let rows = sqlx::query_as::<_, CategoryRow>(
            r#"
            SELECT
                c.id,
                c.name,
                COALESCE(c.color, '') as color,
                COALESCE(c.icon, '') as icon,
                c.parent_id,
                c.category_type::text as category_type
            FROM category c
            WHERE c.user_id = $1
            ORDER BY c.created_at DESC, c.id DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Category::from).collect())
    }
}

pub fn category_type_from_db<T: AsRef<str>>(value: T) -> CategoryType {
    match value.as_ref() {
        "Incoming" => CategoryType::Incoming,
        "Outgoing" => CategoryType::Outgoing,
        "Transfer" => CategoryType::Transfer,
        other => panic!("Unknown category type: {}", other),
    }
}

trait CategoryRequestDbExt {
    fn category_type_to_db(&self) -> String;
}

impl CategoryRequestDbExt for CategoryRequest {
    fn category_type_to_db(&self) -> String {
        match self.category_type {
            CategoryType::Incoming => "Incoming".to_string(),
            CategoryType::Outgoing => "Outgoing".to_string(),
            CategoryType::Transfer => "Transfer".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_category_type_from_db_all_types() {
        assert!(matches!(category_type_from_db("Incoming"), CategoryType::Incoming));
        assert!(matches!(category_type_from_db("Outgoing"), CategoryType::Outgoing));
        assert!(matches!(category_type_from_db("Transfer"), CategoryType::Transfer));
    }

    #[test]
    #[should_panic(expected = "Unknown category type")]
    fn test_category_type_from_db_invalid() {
        category_type_from_db("InvalidType");
    }

    #[test]
    fn test_category_type_to_db() {
        let request = CategoryRequest {
            name: "Test".to_string(),
            color: "#000000".to_string(),
            icon: "icon".to_string(),
            parent_id: None,
            category_type: CategoryType::Incoming,
        };
        assert_eq!(request.category_type_to_db(), "Incoming");

        let request_outgoing = CategoryRequest {
            name: "Test".to_string(),
            color: "#000000".to_string(),
            icon: "icon".to_string(),
            parent_id: None,
            category_type: CategoryType::Outgoing,
        };
        assert_eq!(request_outgoing.category_type_to_db(), "Outgoing");

        let request_transfer = CategoryRequest {
            name: "Test".to_string(),
            color: "#000000".to_string(),
            icon: "icon".to_string(),
            parent_id: None,
            category_type: CategoryType::Transfer,
        };
        assert_eq!(request_transfer.category_type_to_db(), "Transfer");
    }

    #[test]
    fn test_budgeted_diagnostics_conversion_zero_budget() {
        let row = BudgetedCategoryDiagnosticsDbRow {
            id: Uuid::new_v4(),
            name: "Utilities".to_string(),
            color: "#000".to_string(),
            icon: "bolt".to_string(),
            parent_id: None,
            category_type: "Outgoing".to_string(),
            budgeted_value: 0,
            actual_value: 4200,
        };

        let converted = CategoryBudgetedDiagnosticsRow::from(row);
        assert_eq!(converted.budgeted_value, 0);
        assert_eq!(converted.actual_value, 4200);
        assert_eq!(converted.variance_value, 4200);
        assert_eq!(converted.progress_basis_points, 0);
    }

    #[test]
    fn test_budgeted_diagnostics_conversion_negative_actual() {
        let row = BudgetedCategoryDiagnosticsDbRow {
            id: Uuid::new_v4(),
            name: "Refunded".to_string(),
            color: "#111".to_string(),
            icon: "rotate".to_string(),
            parent_id: None,
            category_type: "Outgoing".to_string(),
            budgeted_value: 1000,
            actual_value: -250,
        };

        let converted = CategoryBudgetedDiagnosticsRow::from(row);
        assert_eq!(converted.variance_value, -1250);
        assert_eq!(converted.progress_basis_points, 0);
    }
}
