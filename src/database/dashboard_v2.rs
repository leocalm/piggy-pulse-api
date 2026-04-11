use chrono::NaiveDate;
use uuid::Uuid;

use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::service::dashboard::is_outside_tolerance;

/// Row returned by the current-period query.
#[derive(sqlx::FromRow)]
pub struct CurrentPeriodRow {
    pub spent: i64,
    pub target: i64,
    pub income_target: i64,
    pub days_remaining: i32,
    pub days_in_period: i32,
    pub days_elapsed: i32,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
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
    pub recent_stability: i64,
    pub periods_within_range: i64,
    pub periods_stability: Vec<bool>,
}

/// Row for daily spend sparkline.
#[derive(sqlx::FromRow)]
pub struct DailySpendRow {
    #[allow(dead_code)]
    pub day: NaiveDate,
    pub amount: i64,
}

/// Row returned by cash-flow query.
#[derive(sqlx::FromRow)]
pub struct CashFlowRow {
    pub inflows: i64,
    pub outflows: i64,
}

/// Row returned by spending-trend query.
#[derive(sqlx::FromRow)]
pub struct SpendingTrendRow {
    pub period_id: Uuid,
    pub period_name: String,
    pub total_spend: i64,
}

/// Row returned by top-vendors query.
#[derive(sqlx::FromRow)]
pub struct TopVendorRow {
    pub vendor_id: Uuid,
    pub vendor_name: String,
    pub total_spend: i64,
    pub transaction_count: i64,
}

/// Row returned by uncategorized query.
#[derive(sqlx::FromRow)]
pub struct UncategorizedRow {
    pub id: Uuid,
    pub amount: i64,
    pub occurred_at: NaiveDate,
    pub description: String,
    pub from_account_id: Uuid,
}

/// Row returned by fixed-categories query.
#[derive(sqlx::FromRow)]
pub struct FixedCategoryRow {
    pub category_id: Uuid,
    pub category_name: String,
    pub category_icon: String,
    pub spent: i64,
    pub budgeted: i64,
}

/// One point in the net-position history series (one calendar day).
#[derive(sqlx::FromRow)]
pub struct NetPositionHistoryRow {
    pub date: String,
    pub total: i64,
    pub liquid_amount: i64,
    pub protected_amount: i64,
    pub debt_amount: i64,
}

/// One point in the current-period spending history (one calendar day).
#[derive(sqlx::FromRow)]
pub struct CurrentPeriodHistoryRow {
    pub date: String,
    pub daily_spent: i64,
    pub cumulative_spent: i64,
}

/// One subscription item returned by the dashboard subscriptions query.
#[derive(sqlx::FromRow)]
pub struct SubscriptionDashboardRow {
    pub id: Uuid,
    pub name: String,
    pub billing_amount: i64,
    pub billing_cycle: String,
    pub next_charge_date: NaiveDate,
    pub display_status: String,
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
    LEFT JOIN account fa ON fa.id = t.from_account_id
    LEFT JOIN account ta ON ta.id = t.to_account_id
    WHERE t.user_id = $2
      AND t.occurred_at >= p.start_date
      AND t.occurred_at <= p.end_date
      AND (
          (c.category_type = 'Outgoing' AND (fa.account_type IS NULL OR fa.account_type <> 'Allowance'))
          OR (c.category_type = 'Transfer' AND ta.account_type = 'Allowance')
      )
),
target AS (
    SELECT (
        COALESCE((SELECT SUM(bc2.budgeted_value) FROM budget_category bc2 JOIN category cat2 ON bc2.category_id = cat2.id WHERE bc2.user_id = $2 AND bc2.is_excluded = false AND cat2.category_type = 'Outgoing'), 0)
        + COALESCE((SELECT SUM(a2.spend_limit) FROM account a2 WHERE a2.user_id = $2 AND a2.account_type = 'Allowance' AND a2.is_archived = false AND a2.spend_limit IS NOT NULL), 0)
    )::bigint AS value
),
income_target AS (
    SELECT COALESCE(SUM(bc.budgeted_value), 0)::bigint AS value
    FROM budget_category bc
    JOIN category cat ON bc.category_id = cat.id
    WHERE bc.user_id = $2
      AND bc.is_excluded = false
      AND cat.category_type = 'Incoming'
)
SELECT
    spent.value AS spent,
    target.value AS target,
    income_target.value AS income_target,
    GREATEST(0, (p.end_date - CURRENT_DATE + 1)::int) AS days_remaining,
    GREATEST(1, (p.end_date - p.start_date + 1)::int) AS days_in_period,
    GREATEST(0, LEAST((CURRENT_DATE - p.start_date + 1)::int, (p.end_date - p.start_date + 1)::int)) AS days_elapsed,
    p.start_date,
    p.end_date
FROM period p
CROSS JOIN spent
CROSS JOIN target
CROSS JOIN income_target
            "#,
        )
        .bind(period_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(row)
    }

    /// Fetch daily spend amounts for the period (one entry per calendar day).
    pub async fn get_daily_spend_v2(&self, start_date: NaiveDate, end_date: NaiveDate, user_id: &Uuid) -> Result<Vec<DailySpendRow>, AppError> {
        let rows = sqlx::query_as::<_, DailySpendRow>(
            r#"
SELECT
    gs.day::date AS day,
    COALESCE(SUM(
        CASE
            WHEN c.category_type = 'Outgoing' AND (fa.account_type IS NULL OR fa.account_type <> 'Allowance') THEN t.amount
            WHEN c.category_type = 'Transfer' AND ta.account_type = 'Allowance' THEN t.amount
            ELSE 0
        END
    ), 0)::bigint AS amount
FROM generate_series($1::date, $2::date, '1 day'::interval) AS gs(day)
LEFT JOIN transaction t
    ON t.occurred_at = gs.day::date
    AND t.user_id = $3
LEFT JOIN category c ON c.id = t.category_id
LEFT JOIN account fa ON fa.id = t.from_account_id
LEFT JOIN account ta ON ta.id = t.to_account_id
GROUP BY gs.day
ORDER BY gs.day
            "#,
        )
        .bind(start_date)
        .bind(end_date)
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
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
    COALESCE(SUM(
        CASE
            WHEN ab.account_type = 'CreditCard' THEN -ab.current_balance
            ELSE ab.current_balance
        END
    ), 0)::bigint AS total_net_position,
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
                SELECT (
                    COALESCE((SELECT SUM(bc2.budgeted_value) FROM budget_category bc2 JOIN category cat2 ON bc2.category_id = cat2.id WHERE bc2.user_id = $1 AND bc2.is_excluded = false AND cat2.category_type = 'Outgoing'), 0)
                    + COALESCE((SELECT SUM(a2.spend_limit) FROM account a2 WHERE a2.user_id = $1 AND a2.account_type = 'Allowance' AND a2.is_archived = false AND a2.spend_limit IS NOT NULL), 0)
                )::bigint AS value
            )
            SELECT
                tb.value AS total_budget,
                COALESCE(SUM(
                    CASE
                        WHEN c.category_type = 'Outgoing' AND (fa.account_type IS NULL OR fa.account_type <> 'Allowance') THEN t.amount
                        WHEN c.category_type = 'Transfer' AND ta.account_type = 'Allowance' THEN t.amount
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
            LEFT JOIN account fa ON fa.id = t.from_account_id
            LEFT JOIN account ta ON ta.id = t.to_account_id
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

        // Recent 3 periods for recentStability
        let recent_rows: Vec<&ClosedPeriodRow> = closed_period_rows.iter().take(3).collect();
        let recent_total = recent_rows.len() as i64;
        let recent_within = recent_rows
            .iter()
            .filter(|row| !is_outside_tolerance(row.spent_budget, row.total_budget, tolerance_basis_points))
            .count() as i64;
        let recent_stability = if recent_total == 0 { 0 } else { (recent_within * 100) / recent_total };

        // Recent 6 periods, reversed to chronological order (oldest first)
        let mut periods_stability: Vec<bool> = closed_period_rows
            .iter()
            .take(6)
            .map(|row| !is_outside_tolerance(row.spent_budget, row.total_budget, tolerance_basis_points))
            .collect();
        periods_stability.reverse();

        Ok(BudgetStabilityResult {
            stability,
            recent_stability,
            periods_within_range,
            periods_stability,
        })
    }

    /// Fetch inflows and outflows for a given period.
    pub async fn get_cash_flow_v2(&self, period_id: &Uuid, user_id: &Uuid) -> Result<CashFlowRow, AppError> {
        self.get_budget_period(period_id, user_id).await?;

        let row = sqlx::query_as::<_, CashFlowRow>(
            r#"
SELECT
    COALESCE(SUM(CASE WHEN c.category_type = 'Incoming' THEN t.amount ELSE 0 END), 0)::bigint AS inflows,
    COALESCE(SUM(
        CASE
            WHEN c.category_type = 'Outgoing' AND (fa.account_type IS NULL OR fa.account_type <> 'Allowance') THEN t.amount
            WHEN c.category_type = 'Transfer' AND ta.account_type = 'Allowance' THEN t.amount
            ELSE 0
        END
    ), 0)::bigint AS outflows
FROM transaction t
JOIN category c ON c.id = t.category_id
JOIN budget_period bp ON bp.id = $1 AND bp.user_id = $2
LEFT JOIN account fa ON fa.id = t.from_account_id
LEFT JOIN account ta ON ta.id = t.to_account_id
WHERE t.user_id = $2
  AND t.occurred_at >= bp.start_date
  AND t.occurred_at <= bp.end_date
            "#,
        )
        .bind(period_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(row)
    }

    /// Fetch spend per period for the last N closed/current periods.
    pub async fn get_spending_trend_v2(&self, period_id: &Uuid, user_id: &Uuid, limit: i64) -> Result<Vec<SpendingTrendRow>, AppError> {
        self.get_budget_period(period_id, user_id).await?;

        let rows = sqlx::query_as::<_, SpendingTrendRow>(
            r#"
SELECT
    bp.id AS period_id,
    bp.name AS period_name,
    COALESCE(SUM(
        CASE
            WHEN c.category_type = 'Outgoing' AND (fa.account_type IS NULL OR fa.account_type <> 'Allowance') THEN t.amount
            WHEN c.category_type = 'Transfer' AND ta.account_type = 'Allowance' THEN t.amount
            ELSE 0
        END
    ), 0)::bigint AS total_spend
FROM budget_period bp
LEFT JOIN transaction t
    ON t.user_id = $1
    AND t.occurred_at >= bp.start_date
    AND t.occurred_at <= bp.end_date
LEFT JOIN category c ON c.id = t.category_id
LEFT JOIN account fa ON fa.id = t.from_account_id
LEFT JOIN account ta ON ta.id = t.to_account_id
WHERE bp.user_id = $1
GROUP BY bp.id, bp.name, bp.end_date
ORDER BY bp.end_date DESC
LIMIT $2
            "#,
        )
        .bind(user_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        // Return oldest-first
        let mut rows = rows;
        rows.reverse();
        Ok(rows)
    }

    /// Fetch top vendors by spend within a period.
    pub async fn get_top_vendors_v2(&self, period_id: &Uuid, user_id: &Uuid, limit: i64) -> Result<Vec<TopVendorRow>, AppError> {
        self.get_budget_period(period_id, user_id).await?;

        let rows = sqlx::query_as::<_, TopVendorRow>(
            r#"
SELECT
    v.id AS vendor_id,
    v.name AS vendor_name,
    COALESCE(SUM(t.amount), 0)::bigint AS total_spend,
    COUNT(t.id)::bigint AS transaction_count
FROM vendor v
JOIN transaction t ON t.vendor_id = v.id AND t.user_id = $2
JOIN category c ON c.id = t.category_id
JOIN budget_period bp ON bp.id = $1 AND bp.user_id = $2
LEFT JOIN account fa ON fa.id = t.from_account_id
WHERE v.user_id = $2
  AND t.occurred_at >= bp.start_date
  AND t.occurred_at <= bp.end_date
  AND c.category_type = 'Outgoing'
  AND (fa.account_type IS NULL OR fa.account_type <> 'Allowance')
GROUP BY v.id, v.name
ORDER BY total_spend DESC
LIMIT $3
            "#,
        )
        .bind(period_id)
        .bind(user_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    /// Fetch transactions without a category within a period (up to limit).
    pub async fn get_uncategorized_v2(&self, period_id: &Uuid, user_id: &Uuid, limit: i64) -> Result<Vec<UncategorizedRow>, AppError> {
        self.get_budget_period(period_id, user_id).await?;

        let rows = sqlx::query_as::<_, UncategorizedRow>(
            r#"
SELECT
    t.id,
    t.amount,
    t.occurred_at,
    t.description,
    t.from_account_id
FROM transaction t
JOIN budget_period bp ON bp.id = $1 AND bp.user_id = $2
WHERE t.user_id = $2
  AND t.category_id IS NULL
  AND t.occurred_at >= bp.start_date
  AND t.occurred_at <= bp.end_date
ORDER BY t.occurred_at DESC
LIMIT $3
            "#,
        )
        .bind(period_id)
        .bind(user_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    /// Count total uncategorized transactions within a period.
    pub async fn count_uncategorized_v2(&self, period_id: &Uuid, user_id: &Uuid) -> Result<i64, AppError> {
        #[derive(sqlx::FromRow)]
        struct CountRow {
            count: i64,
        }

        let row = sqlx::query_as::<_, CountRow>(
            r#"
SELECT COUNT(*)::bigint AS count
FROM transaction t
JOIN budget_period bp ON bp.id = $1 AND bp.user_id = $2
WHERE t.user_id = $2
  AND t.category_id IS NULL
  AND t.occurred_at >= bp.start_date
  AND t.occurred_at <= bp.end_date
            "#,
        )
        .bind(period_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.count)
    }

    /// Fetch daily net-position history for the days elapsed within a period.
    /// Returns one row per calendar day from period start up to today (or period end, whichever
    /// is earlier). Returns an empty vec when the user has no accounts.
    pub async fn get_net_position_history_v2(&self, period_id: &Uuid, user_id: &Uuid) -> Result<Vec<NetPositionHistoryRow>, AppError> {
        // Verify period exists (will 404 if not found)
        self.get_budget_period(period_id, user_id).await?;

        let rows = sqlx::query_as::<_, NetPositionHistoryRow>(
            r#"
WITH period AS (
    SELECT start_date, end_date
    FROM budget_period
    WHERE id = $1 AND user_id = $2
),
days AS (
    SELECT d::date AS day
    FROM generate_series(
        (SELECT start_date FROM period),
        LEAST((SELECT end_date FROM period), CURRENT_DATE),
        '1 day'
    ) AS d
),
-- Balance each account carried into the period start (sum of all prior transactions + initial balance)
base_balances AS (
    SELECT
        a.id,
        a.account_type::text AS account_type,
        (a.balance + COALESCE(SUM(
            CASE
                WHEN c.category_type = 'Incoming'                              THEN  t.amount
                WHEN c.category_type = 'Outgoing'                              THEN -t.amount
                WHEN c.category_type = 'Transfer' AND t.from_account_id = a.id THEN -t.amount
                WHEN c.category_type = 'Transfer' AND t.to_account_id   = a.id THEN  t.amount
                ELSE 0
            END
        ), 0))::bigint AS base_balance
    FROM account a
    LEFT JOIN transaction t
        ON (t.from_account_id = a.id OR t.to_account_id = a.id)
        AND t.user_id = $2
        AND t.occurred_at < (SELECT start_date FROM period)
    LEFT JOIN category c ON t.category_id = c.id
    WHERE a.user_id = $2
    GROUP BY a.id, a.account_type, a.balance
),
-- Per-account, per-day delta within the period
daily_deltas AS (
    SELECT
        a.id AS account_id,
        t.occurred_at::date AS day,
        SUM(
            CASE
                WHEN c.category_type = 'Incoming'                              THEN  t.amount
                WHEN c.category_type = 'Outgoing'                              THEN -t.amount
                WHEN c.category_type = 'Transfer' AND t.from_account_id = a.id THEN -t.amount
                WHEN c.category_type = 'Transfer' AND t.to_account_id   = a.id THEN  t.amount
                ELSE 0
            END
        )::bigint AS delta
    FROM account a
    JOIN transaction t
        ON (t.from_account_id = a.id OR t.to_account_id = a.id)
        AND t.user_id = $2
    JOIN category c ON t.category_id = c.id
    CROSS JOIN period
    WHERE a.user_id = $2
      AND t.occurred_at >= period.start_date
      AND t.occurred_at <= LEAST(period.end_date, CURRENT_DATE)
    GROUP BY a.id, t.occurred_at::date
),
-- Running balance per account per day
running_balances AS (
    SELECT
        bb.id AS account_id,
        bb.account_type,
        d.day,
        (bb.base_balance + SUM(COALESCE(dd.delta, 0)) OVER (
            PARTITION BY bb.id
            ORDER BY d.day
            ROWS UNBOUNDED PRECEDING
        ))::bigint AS balance
    FROM base_balances bb
    CROSS JOIN days d
    LEFT JOIN daily_deltas dd ON dd.account_id = bb.id AND dd.day = d.day
)
SELECT
    to_char(d.day, 'YYYY-MM-DD')                                                                  AS date,
    COALESCE(SUM(rb.balance), 0)::bigint                                                          AS total,
    COALESCE(SUM(CASE WHEN rb.account_type IN ('Checking', 'Wallet', 'Allowance') THEN rb.balance ELSE 0 END), 0)::bigint AS liquid_amount,
    COALESCE(SUM(CASE WHEN rb.account_type = 'Savings'     THEN rb.balance ELSE 0 END), 0)::bigint AS protected_amount,
    COALESCE(SUM(CASE WHEN rb.account_type = 'CreditCard'  THEN rb.balance ELSE 0 END), 0)::bigint AS debt_amount
FROM days d
LEFT JOIN running_balances rb ON rb.day = d.day
GROUP BY d.day
ORDER BY d.day
            "#,
        )
        .bind(period_id)
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    /// Fetch daily spending history for the elapsed days within a period.
    /// Returns one row per calendar day from period start up to today (or period end, whichever
    /// is earlier), with the daily amount spent and a cumulative running total.
    pub async fn get_current_period_history_v2(&self, period_id: &Uuid, user_id: &Uuid) -> Result<Vec<CurrentPeriodHistoryRow>, AppError> {
        // Verify period exists (will 404 if not found)
        self.get_budget_period(period_id, user_id).await?;

        let rows = sqlx::query_as::<_, CurrentPeriodHistoryRow>(
            r#"
WITH period AS (
    SELECT start_date, end_date
    FROM budget_period
    WHERE id = $1 AND user_id = $2
),
days AS (
    SELECT d::date AS day
    FROM generate_series(
        (SELECT start_date FROM period),
        LEAST((SELECT end_date FROM period), CURRENT_DATE),
        '1 day'
    ) AS d
),
daily_spend AS (
    SELECT
        days.day,
        COALESCE(SUM(
            CASE
                WHEN c.category_type = 'Outgoing' AND (fa.account_type IS NULL OR fa.account_type <> 'Allowance') THEN t.amount
                WHEN c.category_type = 'Transfer' AND ta.account_type = 'Allowance' THEN t.amount
                ELSE 0
            END
        ), 0)::bigint AS daily_spent
    FROM days
    LEFT JOIN transaction t
        ON t.occurred_at = days.day
        AND t.user_id = $2
    LEFT JOIN category c ON c.id = t.category_id
    LEFT JOIN account fa ON fa.id = t.from_account_id
    LEFT JOIN account ta ON ta.id = t.to_account_id
    GROUP BY days.day
)
SELECT
    to_char(ds.day, 'YYYY-MM-DD') AS date,
    ds.daily_spent,
    COALESCE(SUM(ds.daily_spent) OVER (ORDER BY ds.day ROWS UNBOUNDED PRECEDING), 0)::bigint AS cumulative_spent
FROM daily_spend ds
ORDER BY ds.day
            "#,
        )
        .bind(period_id)
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    /// Fetch fixed categories with their status for a period.
    pub async fn get_fixed_categories_v2(&self, period_id: &Uuid, user_id: &Uuid) -> Result<Vec<FixedCategoryRow>, AppError> {
        self.get_budget_period(period_id, user_id).await?;

        let rows = sqlx::query_as::<_, FixedCategoryRow>(
            r#"
SELECT
    c.id AS category_id,
    c.name AS category_name,
    COALESCE(c.icon, '') AS category_icon,
    COALESCE(SUM(CASE
        WHEN t.occurred_at >= bp.start_date AND t.occurred_at <= bp.end_date THEN t.amount
        ELSE 0
    END), 0)::bigint AS spent,
    COALESCE(bc.budgeted_value, 0)::bigint AS budgeted
FROM category c
JOIN budget_period bp ON bp.id = $1 AND bp.user_id = $2
LEFT JOIN transaction t ON t.category_id = c.id AND t.user_id = $2
LEFT JOIN budget_category bc ON bc.category_id = c.id AND bc.user_id = $2
WHERE c.user_id = $2
  AND c.category_type = 'Outgoing'
  AND c.behavior = 'fixed'
  AND c.is_archived = false
GROUP BY c.id, c.name, c.icon, bc.budgeted_value, bp.start_date, bp.end_date
ORDER BY c.name
            "#,
        )
        .bind(period_id)
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    pub async fn get_subscriptions_v2(&self, period_id: &Uuid, user_id: &Uuid) -> Result<Vec<SubscriptionDashboardRow>, AppError> {
        self.get_budget_period(period_id, user_id).await?;

        let rows = sqlx::query_as::<_, SubscriptionDashboardRow>(
            r#"
SELECT
    s.id                                                                  AS id,
    s.name                                                                AS name,
    s.billing_amount                                                      AS billing_amount,
    s.billing_cycle::text                                                 AS billing_cycle,
    s.next_charge_date                                                    AS next_charge_date,
    CASE
        WHEN sbe.id IS NOT NULL               THEN 'charged'
        WHEN s.next_charge_date = CURRENT_DATE THEN 'today'
        ELSE 'upcoming'
    END                                                                   AS display_status
FROM subscription s
JOIN budget_period bp ON bp.id = $1 AND bp.user_id = $2
LEFT JOIN LATERAL (
    SELECT id
    FROM subscription_billing_event
    WHERE subscription_id = s.id
      AND date >= bp.start_date
      AND date <= bp.end_date
    LIMIT 1
) sbe ON true
WHERE s.user_id = $2
  AND s.status  = 'active'
  AND (
      sbe.id IS NOT NULL
      OR (s.next_charge_date >= bp.start_date AND s.next_charge_date <= bp.end_date)
  )
ORDER BY
    CASE
        WHEN sbe.id IS NOT NULL               THEN 0
        WHEN s.next_charge_date = CURRENT_DATE THEN 1
        ELSE 2
    END,
    s.next_charge_date ASC
            "#,
        )
        .bind(period_id)
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }
}
