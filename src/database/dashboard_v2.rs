use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::service::dashboard::is_outside_tolerance;
use uuid::Uuid;

/// Row returned by the current-period query.
#[derive(sqlx::FromRow)]
pub struct CurrentPeriodRow {
    pub spent: i64,
    pub target: i64,
    pub days_remaining: i32,
    pub days_in_period: i32,
    pub days_elapsed: i32,
}

/// Row returned by the net-position query.
#[derive(sqlx::FromRow)]
pub struct NetPositionRow {
    pub total_net_position: i64,
    pub change_this_period: i64,
    pub liquid_balance: i64,
    pub protected_balance: i64,
    pub debt_balance: i64,
    pub account_count: i64,
}

/// Row returned per closed period for stability calculation.
#[derive(sqlx::FromRow)]
struct ClosedPeriodRow {
    total_budget: i64,
    spent_budget: i64,
}

/// Result of the budget stability calculation.
pub struct BudgetStabilityResult {
    pub stability: i64,
    pub periods_within_range: i64,
    pub periods_stability: Vec<bool>,
}

impl PostgresRepository {
    /// Fetch current-period dashboard data for a given period.
    /// Returns `AppError::NotFound` if the period does not exist for this user.
    pub async fn get_current_period_dashboard(&self, period_id: &Uuid, user_id: &Uuid) -> Result<CurrentPeriodRow, AppError> {
        let row = sqlx::query_as::<_, CurrentPeriodRow>(
            r#"
WITH period AS (
    SELECT id, start_date, end_date
    FROM budget_period
    WHERE id = $1 AND user_id = $2
),
spent AS (
    SELECT COALESCE(SUM(t.amount), 0)::bigint AS value
    FROM transaction t
    JOIN category c ON c.id = t.category_id
    JOIN period p ON TRUE
    WHERE t.user_id = $2
      AND c.category_type = 'Outgoing'
      AND t.occurred_at >= p.start_date
      AND t.occurred_at <= p.end_date
),
target AS (
    SELECT COALESCE(SUM(bc.budgeted_value), 0)::bigint AS value
    FROM budget_category bc
    WHERE bc.user_id = $2
      AND bc.is_excluded = false
)
SELECT
    spent.value AS spent,
    target.value AS target,
    GREATEST(0, (p.end_date - CURRENT_DATE)::int) AS days_remaining,
    GREATEST(1, (p.end_date - p.start_date)::int) AS days_in_period,
    GREATEST(0, LEAST((CURRENT_DATE - p.start_date)::int, (p.end_date - p.start_date)::int)) AS days_elapsed
FROM period p
CROSS JOIN spent
CROSS JOIN target
            "#,
        )
        .bind(period_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(row)
    }

    /// Fetch net-position dashboard data for a given period.
    /// Returns `AppError::NotFound` if the period does not exist for this user.
    pub async fn get_net_position_v2(&self, period_id: &Uuid, user_id: &Uuid) -> Result<NetPositionRow, AppError> {
        // Verify period exists (will 404 if not found)
        self.get_budget_period(period_id, user_id).await?;

        let row = sqlx::query_as::<_, NetPositionRow>(
            r#"
WITH account_balances AS (
    SELECT
        a.id,
        a.account_type::text AS account_type,
        (
            a.balance + COALESCE(
                SUM(
                    CASE
                        WHEN c.category_type = 'Incoming' THEN t.amount
                        WHEN c.category_type = 'Outgoing' THEN -t.amount
                        WHEN c.category_type = 'Transfer' AND t.from_account_id = a.id THEN -t.amount
                        WHEN c.category_type = 'Transfer' AND t.to_account_id = a.id THEN t.amount
                        ELSE 0
                    END
                ),
                0
            )
        )::bigint AS current_balance
    FROM account a
    LEFT JOIN transaction t
        ON (t.from_account_id = a.id OR t.to_account_id = a.id)
        AND t.user_id = $1
    LEFT JOIN category c ON c.id = t.category_id
    WHERE a.user_id = $1
    GROUP BY a.id, a.account_type, a.balance
),
period_change AS (
    SELECT
        COALESCE(
            SUM(
                CASE
                    WHEN c.category_type = 'Incoming' THEN t.amount
                    WHEN c.category_type = 'Outgoing' THEN -t.amount
                    ELSE 0
                END
            ),
            0
        )::bigint AS value
    FROM transaction t
    JOIN category c ON c.id = t.category_id
    JOIN budget_period bp ON bp.id = $2 AND bp.user_id = $1
    WHERE t.user_id = $1
      AND t.occurred_at >= bp.start_date
      AND t.occurred_at <= LEAST(bp.end_date, CURRENT_DATE)
)
SELECT
    COALESCE(SUM(ab.current_balance), 0)::bigint AS total_net_position,
    (SELECT value FROM period_change)::bigint AS change_this_period,
    COALESCE(
        SUM(
            CASE
                WHEN ab.account_type IN ('Checking', 'Wallet', 'Allowance') THEN ab.current_balance
                ELSE 0
            END
        ),
        0
    )::bigint AS liquid_balance,
    COALESCE(
        SUM(
            CASE
                WHEN ab.account_type = 'Savings' THEN ab.current_balance
                ELSE 0
            END
        ),
        0
    )::bigint AS protected_balance,
    COALESCE(
        SUM(
            CASE
                WHEN ab.account_type = 'CreditCard' THEN ab.current_balance
                ELSE 0
            END
        ),
        0
    )::bigint AS debt_balance,
    COUNT(ab.id)::bigint AS account_count
FROM account_balances ab
            "#,
        )
        .bind(user_id)
        .bind(period_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(row)
    }

    /// Fetch budget stability data.
    /// Uses the same logic as V1: tolerance from settings, closed periods only.
    pub async fn get_budget_stability_v2(&self, period_id: &Uuid, user_id: &Uuid) -> Result<BudgetStabilityResult, AppError> {
        // Verify period exists (will 404 if not found)
        self.get_budget_period(period_id, user_id).await?;

        #[derive(sqlx::FromRow)]
        struct ToleranceRow {
            budget_stability_tolerance_basis_points: i32,
        }

        let tolerance_row = sqlx::query_as::<_, ToleranceRow>(
            r#"
            SELECT budget_stability_tolerance_basis_points
            FROM settings
            WHERE user_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        let tolerance_basis_points = tolerance_row.map(|row| row.budget_stability_tolerance_basis_points).unwrap_or(1000);

        let closed_period_rows = sqlx::query_as::<_, ClosedPeriodRow>(
            r#"
            WITH total_budget AS (
                SELECT COALESCE(SUM(budgeted_value), 0)::bigint AS value
                FROM budget_category
                WHERE user_id = $1
            )
            SELECT
                tb.value AS total_budget,
                COALESCE(SUM(
                    CASE
                        WHEN c.category_type = 'Outgoing' THEN t.amount
                        ELSE 0
                    END
                ), 0)::bigint AS spent_budget
            FROM budget_period bp
            CROSS JOIN total_budget tb
            LEFT JOIN transaction t
                ON t.user_id = $1
                AND t.occurred_at >= bp.start_date
                AND t.occurred_at <= bp.end_date
            LEFT JOIN category c
                ON c.id = t.category_id
            WHERE bp.user_id = $1
                AND bp.end_date < CURRENT_DATE
            GROUP BY bp.id, bp.end_date, tb.value
            ORDER BY bp.end_date DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        let total_closed = closed_period_rows.len() as i64;
        let periods_within_range = closed_period_rows
            .iter()
            .filter(|row| !is_outside_tolerance(row.spent_budget, row.total_budget, tolerance_basis_points))
            .count() as i64;

        let stability = if total_closed == 0 { 0 } else { (periods_within_range * 100) / total_closed };

        // Recent 6 periods, reversed to chronological order (oldest first)
        let mut periods_stability: Vec<bool> = closed_period_rows
            .iter()
            .take(6)
            .map(|row| !is_outside_tolerance(row.spent_budget, row.total_budget, tolerance_basis_points))
            .collect();
        periods_stability.reverse();

        Ok(BudgetStabilityResult {
            stability,
            periods_within_range,
            periods_stability,
        })
    }
}
