use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::models::category::{Category, CategoryRequest, CategoryStats, CategoryType, CategoryWithStats, difference_vs_average_percentage};
use crate::models::pagination::CursorParams;
use chrono::{DateTime, Utc};
use uuid::Uuid;

// Intermediate struct for sqlx query results with category_type as text
#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct CategoryRow {
    id: Uuid,
    user_id: Uuid,
    name: String,
    color: String,
    icon: String,
    parent_id: Option<Uuid>,
    category_type: String,
    created_at: DateTime<Utc>,
}

impl From<CategoryRow> for Category {
    fn from(row: CategoryRow) -> Self {
        Category {
            id: row.id,
            user_id: row.user_id,
            name: row.name,
            color: row.color,
            icon: row.icon,
            parent_id: row.parent_id,
            category_type: category_type_from_db(&row.category_type),
            created_at: row.created_at,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct CategoryWithStatsRow {
    id: Uuid,
    user_id: Uuid,
    name: String,
    color: String,
    icon: String,
    parent_id: Option<Uuid>,
    category_type: String,
    created_at: DateTime<Utc>,
    used_this_month: i64,
    average_monthly_usage: i64,
    transaction_count: i64,
}

impl From<CategoryWithStatsRow> for CategoryWithStats {
    fn from(row: CategoryWithStatsRow) -> Self {
        CategoryWithStats {
            category: Category {
                id: row.id,
                user_id: row.user_id,
                name: row.name,
                color: row.color,
                icon: row.icon,
                parent_id: row.parent_id,
                category_type: category_type_from_db(&row.category_type),
                created_at: row.created_at,
            },
            stats: CategoryStats {
                used_this_month: row.used_this_month,
                difference_vs_average_percentage: difference_vs_average_percentage(row.used_this_month, row.average_monthly_usage),
                transaction_count: row.transaction_count,
            },
        }
    }
}

impl PostgresRepository {
    pub async fn create_category(&self, request: &CategoryRequest, user_id: &Uuid) -> Result<Category, AppError> {
        let row = sqlx::query_as::<_, CategoryRow>(
            r#"
            INSERT INTO category (user_id, name, color, icon, parent_id, category_type)
            VALUES ($1, $2, $3, $4, $5, $6::text::category_type)
            RETURNING
                id,
                user_id,
                name,
                COALESCE(color, '') as color,
                COALESCE(icon, '') as icon,
                parent_id,
                category_type::text as category_type,
                created_at
            "#,
        )
        .bind(user_id)
        .bind(&request.name)
        .bind(&request.color)
        .bind(&request.icon)
        .bind(request.parent_id)
        .bind(request.category_type_to_db())
        .fetch_one(&self.pool)
        .await?;

        Ok(Category::from(row))
    }

    pub async fn get_category_by_id(&self, id: &Uuid, user_id: &Uuid) -> Result<Option<Category>, AppError> {
        let row = sqlx::query_as::<_, CategoryRow>(
            r#"
            SELECT
                id,
                user_id,
                name,
                COALESCE(color, '') as color,
                COALESCE(icon, '') as icon,
                parent_id,
                category_type::text as category_type,
                created_at
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

    pub async fn list_categories(&self, params: &CursorParams, user_id: &Uuid) -> Result<Vec<CategoryWithStats>, AppError> {
        let rows = if let Some(cursor) = params.cursor {
            sqlx::query_as::<_, CategoryWithStatsRow>(
                r#"
WITH current_month AS (
    SELECT
        date_trunc('month', CURRENT_DATE)::date AS start_date,
        (date_trunc('month', CURRENT_DATE) + INTERVAL '1 month')::date AS end_date
),
monthly_totals AS (
    SELECT
        t.category_id,
        date_trunc('month', t.occurred_at)::date AS month_start,
        SUM(t.amount)::bigint AS monthly_amount
    FROM transaction t
    WHERE t.user_id = $1
    GROUP BY t.category_id, date_trunc('month', t.occurred_at)
),
average_totals AS (
    SELECT
        category_id,
        COALESCE(AVG(monthly_amount), 0)::bigint AS avg_monthly_amount
    FROM monthly_totals
    GROUP BY category_id
),
current_month_totals AS (
    SELECT
        t.category_id,
        COALESCE(SUM(t.amount), 0)::bigint AS used_this_month
    FROM transaction t
    CROSS JOIN current_month cm
    WHERE t.user_id = $1
      AND t.occurred_at >= cm.start_date
      AND t.occurred_at < cm.end_date
    GROUP BY t.category_id
),
transaction_counts AS (
    SELECT
        t.category_id,
        COUNT(*)::bigint AS transaction_count
    FROM transaction t
    WHERE t.user_id = $1
    GROUP BY t.category_id
)
SELECT
    c.id,
    c.user_id,
    c.name,
    COALESCE(c.color, '') as color,
    COALESCE(c.icon, '') as icon,
    c.parent_id,
    c.category_type::text as category_type,
    c.created_at,
    COALESCE(cmt.used_this_month, 0) AS used_this_month,
    COALESCE(at.avg_monthly_amount, 0) AS average_monthly_usage,
    COALESCE(tc.transaction_count, 0) AS transaction_count
FROM category c
LEFT JOIN current_month_totals cmt ON c.id = cmt.category_id
LEFT JOIN average_totals at ON c.id = at.category_id
LEFT JOIN transaction_counts tc ON c.id = tc.category_id
WHERE c.user_id = $1
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
            sqlx::query_as::<_, CategoryWithStatsRow>(
                r#"
WITH current_month AS (
    SELECT
        date_trunc('month', CURRENT_DATE)::date AS start_date,
        (date_trunc('month', CURRENT_DATE) + INTERVAL '1 month')::date AS end_date
),
monthly_totals AS (
    SELECT
        t.category_id,
        date_trunc('month', t.occurred_at)::date AS month_start,
        SUM(t.amount)::bigint AS monthly_amount
    FROM transaction t
    WHERE t.user_id = $1
    GROUP BY t.category_id, date_trunc('month', t.occurred_at)
),
average_totals AS (
    SELECT
        category_id,
        COALESCE(AVG(monthly_amount), 0)::bigint AS avg_monthly_amount
    FROM monthly_totals
    GROUP BY category_id
),
current_month_totals AS (
    SELECT
        t.category_id,
        COALESCE(SUM(t.amount), 0)::bigint AS used_this_month
    FROM transaction t
    CROSS JOIN current_month cm
    WHERE t.user_id = $1
      AND t.occurred_at >= cm.start_date
      AND t.occurred_at < cm.end_date
    GROUP BY t.category_id
),
transaction_counts AS (
    SELECT
        t.category_id,
        COUNT(*)::bigint AS transaction_count
    FROM transaction t
    WHERE t.user_id = $1
    GROUP BY t.category_id
)
SELECT
    c.id,
    c.user_id,
    c.name,
    COALESCE(c.color, '') as color,
    COALESCE(c.icon, '') as icon,
    c.parent_id,
    c.category_type::text as category_type,
    c.created_at,
    COALESCE(cmt.used_this_month, 0) AS used_this_month,
    COALESCE(at.avg_monthly_amount, 0) AS average_monthly_usage,
    COALESCE(tc.transaction_count, 0) AS transaction_count
FROM category c
LEFT JOIN current_month_totals cmt ON c.id = cmt.category_id
LEFT JOIN average_totals at ON c.id = at.category_id
LEFT JOIN transaction_counts tc ON c.id = tc.category_id
WHERE c.user_id = $1
ORDER BY c.created_at DESC, c.id DESC
LIMIT $2
                "#,
            )
            .bind(user_id)
            .bind(params.fetch_limit())
            .fetch_all(&self.pool)
            .await?
        };

        Ok(rows.into_iter().map(CategoryWithStats::from).collect())
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
        let row = sqlx::query_as::<_, CategoryRow>(
            r#"
            UPDATE category
            SET name = $1, color = $2, icon = $3, parent_id = $4, category_type = $5::text::category_type
            WHERE id = $6 AND user_id = $7
            RETURNING
                id,
                user_id,
                name,
                COALESCE(color, '') as color,
                COALESCE(icon, '') as icon,
                parent_id,
                category_type::text as category_type,
                created_at
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
        .await?;

        Ok(Category::from(row))
    }

    pub async fn list_categories_not_in_budget(&self, params: &CursorParams, user_id: &Uuid) -> Result<Vec<Category>, AppError> {
        let rows = if let Some(cursor) = params.cursor {
            sqlx::query_as::<_, CategoryRow>(
                r#"
                SELECT
                    c.id,
                    c.user_id,
                    c.name,
                    COALESCE(c.color, '') as color,
                    COALESCE(c.icon, '') as icon,
                    c.parent_id,
                    c.category_type::text as category_type,
                    c.created_at
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
                    c.user_id,
                    c.name,
                    COALESCE(c.color, '') as color,
                    COALESCE(c.icon, '') as icon,
                    c.parent_id,
                    c.category_type::text as category_type,
                    c.created_at
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
}
