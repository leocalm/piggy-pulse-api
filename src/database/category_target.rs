use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::models::category_target::{CategoryTargetRow, CategoryTargetsResponse, TargetEntry};
use uuid::Uuid;

#[derive(Debug, sqlx::FromRow)]
struct RawTargetRow {
    category_id: Uuid,
    category_name: String,
    category_type: String,
    category_icon: String,
    category_color: String,
    is_archived: bool,
    is_parent: bool,
    parent_category_name: Option<String>,
    current_target: Option<i32>,
    previous_target: Option<i32>,
    is_excluded: bool,
    projected_variance_basis_points: Option<i32>,
}

impl From<RawTargetRow> for CategoryTargetRow {
    fn from(row: RawTargetRow) -> Self {
        let exclusion_reason = if row.is_excluded {
            if row.category_type == "Transfer" {
                Some("This category never participates in target tracking.".to_string())
            } else {
                Some("Category is intentionally tracked without target comparison.".to_string())
            }
        } else {
            None
        };

        CategoryTargetRow {
            id: row.category_id.to_string(),
            category_id: row.category_id,
            category_name: row.category_name,
            category_type: row.category_type,
            category_icon: row.category_icon,
            category_color: row.category_color,
            is_archived: row.is_archived,
            is_parent: row.is_parent,
            parent_category_name: row.parent_category_name,
            current_target: row.current_target,
            previous_target: row.previous_target,
            is_excluded: row.is_excluded,
            exclusion_reason,
            projected_variance_basis_points: row.projected_variance_basis_points,
        }
    }
}

impl PostgresRepository {
    /// Fetch all category targets for a given period.
    ///
    /// For each non-archived, non-Transfer category:
    /// - current_target: the budgeted_value from budget_category (if it exists and is_excluded=false)
    /// - previous_target: budgeted_value from the immediately preceding period
    /// - projected_variance: basis points showing how much the actual spend deviates from target
    pub async fn get_category_targets(
        &self,
        period_id: &Uuid,
        user_id: &Uuid,
    ) -> Result<CategoryTargetsResponse, AppError> {
        // 1. Get period info
        #[derive(sqlx::FromRow)]
        struct PeriodInfo {
            id: Uuid,
            name: String,
            start_date: chrono::NaiveDate,
            end_date: chrono::NaiveDate,
        }

        let period = sqlx::query_as::<_, PeriodInfo>(
            r#"
            SELECT id, name, start_date, end_date
            FROM budget_period
            WHERE id = $1 AND user_id = $2
            "#,
        )
        .bind(period_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Budget period not found".to_string()))?;

        // Calculate period progress
        let today = chrono::Utc::now().date_naive();
        let total_days = (period.end_date - period.start_date).num_days().max(1);
        let elapsed_days = (today - period.start_date).num_days().clamp(0, total_days);
        let progress_percent = ((elapsed_days * 100) / total_days) as i32;

        // 2. Find the previous period (the one with end_date <= this period's start_date)
        let previous_period_id: Option<Uuid> = sqlx::query_scalar(
            r#"
            SELECT id FROM budget_period
            WHERE user_id = $1 AND end_date <= $2
            ORDER BY end_date DESC
            LIMIT 1
            "#,
        )
        .bind(user_id)
        .bind(period.start_date)
        .fetch_optional(&self.pool)
        .await?;

        // 3. Get all non-archived categories with their target data
        let rows = sqlx::query_as::<_, RawTargetRow>(
            r#"
            WITH period_transactions AS (
                SELECT
                    t.category_id,
                    SUM(t.amount) as total_amount
                FROM transaction t
                WHERE t.user_id = $1
                    AND t.occurred_at >= $3
                    AND t.occurred_at < $4
                GROUP BY t.category_id
            ),
            current_targets AS (
                SELECT
                    bc.category_id,
                    bc.budgeted_value,
                    bc.is_excluded
                FROM budget_category bc
                WHERE bc.user_id = $1
            ),
            previous_targets AS (
                SELECT
                    bc.category_id,
                    bc.budgeted_value
                FROM budget_category bc
                WHERE bc.user_id = $1
                    AND ($5::uuid IS NOT NULL)
            )
            SELECT
                c.id as category_id,
                c.name as category_name,
                c.category_type::text as category_type,
                COALESCE(c.icon, '') as category_icon,
                COALESCE(c.color, '') as category_color,
                c.is_archived,
                (c.parent_id IS NULL) as is_parent,
                parent.name as parent_category_name,
                ct.budgeted_value as current_target,
                pt.budgeted_value as previous_target,
                COALESCE(ct.is_excluded, FALSE) as is_excluded,
                CASE
                    WHEN ct.budgeted_value IS NOT NULL AND ct.budgeted_value > 0 THEN
                        ((COALESCE(ptx.total_amount, 0) * 10000) / ct.budgeted_value)::integer
                    ELSE NULL
                END as projected_variance_basis_points
            FROM category c
            LEFT JOIN category parent ON c.parent_id = parent.id
            LEFT JOIN current_targets ct ON ct.category_id = c.id
            LEFT JOIN previous_targets pt ON pt.category_id = c.id
            LEFT JOIN period_transactions ptx ON ptx.category_id = c.id
            WHERE c.user_id = $1
                AND c.category_type != 'Transfer'
            ORDER BY c.name ASC
            "#,
        )
        .bind(user_id)
        .bind(period_id)
        .bind(period.start_date)
        .bind(period.end_date)
        .bind(previous_period_id)
        .fetch_all(&self.pool)
        .await?;

        let mut outgoing: Vec<CategoryTargetRow> = Vec::new();
        let mut incoming: Vec<CategoryTargetRow> = Vec::new();
        let mut excluded: Vec<CategoryTargetRow> = Vec::new();
        let mut total_targeted: i64 = 0;
        let mut targeted_count = 0;
        let total_categories = rows.len() as i32;

        for raw in rows {
            let row = CategoryTargetRow::from(raw);

            if row.is_excluded {
                excluded.push(row);
            } else {
                if let Some(target) = row.current_target {
                    total_targeted += target as i64;
                    targeted_count += 1;
                }
                match row.category_type.as_str() {
                    "Outgoing" => outgoing.push(row),
                    "Incoming" => incoming.push(row),
                    _ => {}
                }
            }
        }

        Ok(CategoryTargetsResponse {
            period_id: period.id,
            period_name: period.name,
            period_start_date: period.start_date.to_string(),
            period_end_date: period.end_date.to_string(),
            period_progress_percent: progress_percent,
            total_targeted,
            total_categories,
            targeted_categories: targeted_count,
            outgoing_targets: outgoing,
            incoming_targets: incoming,
            excluded_categories: excluded,
        })
    }

    /// Batch upsert category targets: for each target entry, create or update the budget_category row
    pub async fn batch_upsert_targets(
        &self,
        targets: &[TargetEntry],
        user_id: &Uuid,
    ) -> Result<(), AppError> {
        let mut tx = self.pool.begin().await?;

        for entry in targets {
            sqlx::query(
                r#"
                INSERT INTO budget_category (user_id, category_id, budgeted_value, is_excluded)
                VALUES ($1, $2, $3, FALSE)
                ON CONFLICT (user_id, category_id)
                DO UPDATE SET budgeted_value = EXCLUDED.budgeted_value
                "#,
            )
            .bind(user_id)
            .bind(entry.category_id)
            .bind(entry.budgeted_value)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    /// Mark a category as excluded from target tracking
    pub async fn exclude_category_from_targets(
        &self,
        category_id: &Uuid,
        user_id: &Uuid,
    ) -> Result<(), AppError> {
        sqlx::query(
            r#"
            INSERT INTO budget_category (user_id, category_id, budgeted_value, is_excluded)
            VALUES ($1, $2, 0, TRUE)
            ON CONFLICT (user_id, category_id)
            DO UPDATE SET is_excluded = TRUE
            "#,
        )
        .bind(user_id)
        .bind(category_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Re-include a previously excluded category in target tracking
    pub async fn include_category_in_targets(
        &self,
        category_id: &Uuid,
        user_id: &Uuid,
    ) -> Result<(), AppError> {
        sqlx::query(
            r#"
            UPDATE budget_category
            SET is_excluded = FALSE
            WHERE category_id = $1 AND user_id = $2
            "#,
        )
        .bind(category_id)
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
