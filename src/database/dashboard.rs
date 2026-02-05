use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::models::dashboard::{BudgetPerDayResponse, MonthProgressResponse, MonthlyBurnInResponse, SpentPerCategoryResponse, TotalAssetsResponse};

use chrono::NaiveDate;
use uuid::Uuid;

impl PostgresRepository {
    pub async fn balance_per_day(&self, budget_period_id: &Uuid, user_id: &Uuid) -> Result<Vec<BudgetPerDayResponse>, AppError> {
        #[derive(sqlx::FromRow)]
        struct BalancePerDayRow {
            account_name: String,
            date: String,
            balance: i32,
        }

        let rows = sqlx::query_as::<_, BalancePerDayRow>(
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
base_balances AS (
    SELECT
        a.id,
        a.balance + COALESCE(SUM(
            CASE
                WHEN c.category_type = 'Incoming'                              THEN  t.amount
                WHEN c.category_type = 'Outgoing'                              THEN -t.amount
                WHEN c.category_type = 'Transfer' AND t.from_account_id = a.id THEN -t.amount
                WHEN c.category_type = 'Transfer' AND t.to_account_id   = a.id THEN  t.amount
                ELSE 0
            END
        ), 0) AS base_balance
    FROM account a
    LEFT JOIN transaction t ON (t.from_account_id = a.id OR t.to_account_id = a.id)
                            AND t.occurred_at < (SELECT start_date FROM period)
                            AND t.user_id = $2
    LEFT JOIN category c    ON t.category_id = c.id
    WHERE a.user_id = $2
    GROUP BY a.id, a.balance
),
daily_totals AS (
    SELECT
        a.id AS account_id,
        t.occurred_at::date AS occurred_date,
        SUM(
            CASE
                WHEN c.category_type = 'Incoming'                              THEN  t.amount
                WHEN c.category_type = 'Outgoing'                              THEN -t.amount
                WHEN c.category_type = 'Transfer' AND t.from_account_id = a.id THEN -t.amount
                WHEN c.category_type = 'Transfer' AND t.to_account_id   = a.id THEN  t.amount
                ELSE 0
            END
        ) AS daily_amount
    FROM account a
    JOIN transaction t  ON t.from_account_id = a.id OR t.to_account_id = a.id
    JOIN category   c   ON t.category_id = c.id
    CROSS JOIN period
    WHERE a.user_id = $2
      AND t.user_id = $2
      AND t.occurred_at >= period.start_date
      AND t.occurred_at <= LEAST(period.end_date, CURRENT_DATE)
    GROUP BY a.id, t.occurred_at::date
)
SELECT
    a.name                           AS account_name,
    to_char(d.day, 'YYYY-MM-DD')     AS date,
    (bb.base_balance + SUM(COALESCE(dt.daily_amount, 0)) OVER (
        PARTITION BY a.id
        ORDER BY d.day
        ROWS UNBOUNDED PRECEDING
    ))::int                          AS balance
FROM account a
JOIN  base_balances bb ON bb.id = a.id
CROSS JOIN days d
LEFT JOIN daily_totals dt ON dt.account_id = a.id AND dt.occurred_date = d.day
WHERE a.user_id = $2
ORDER BY a.name, d.day
            "#,
        )
        .bind(budget_period_id)
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| BudgetPerDayResponse {
                account_name: row.account_name,
                date: row.date,
                balance: row.balance,
            })
            .collect())
    }

    pub async fn spent_per_category(&self, budget_period_id: &Uuid, user_id: &Uuid) -> Result<Vec<SpentPerCategoryResponse>, AppError> {
        #[derive(sqlx::FromRow)]
        struct SpentPerCategoryRow {
            category_name: String,
            budgeted_value: i32,
            amount_spent: i32,
        }

        let rows = sqlx::query_as::<_, SpentPerCategoryRow>(
            r#"
WITH period_transactions AS (
    SELECT t.category_id, t.amount
    FROM transaction t
    CROSS JOIN budget_period bp
    WHERE bp.id        = $1
      AND bp.user_id   = $2
      AND t.user_id    = $2
      AND t.occurred_at >= bp.start_date
      AND t.occurred_at <= bp.end_date
)
SELECT c.name                                AS category_name,
       bc.budgeted_value,
       COALESCE(SUM(pt.amount), 0)::int      AS amount_spent
FROM budget_category bc
JOIN  category c                ON c.id  = bc.category_id
LEFT JOIN period_transactions pt ON c.id = pt.category_id
WHERE bc.user_id = $2
  AND c.category_type = 'Outgoing'
GROUP BY c.name, bc.budgeted_value
            "#,
        )
        .bind(budget_period_id)
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| SpentPerCategoryResponse {
                category_name: row.category_name,
                budgeted_value: row.budgeted_value,
                amount_spent: row.amount_spent,
                percentage_spent: 0,
            })
            .collect())
    }

    pub async fn monthly_burn_in(&self, budget_period_id: &Uuid, user_id: &Uuid) -> Result<MonthlyBurnInResponse, AppError> {
        let row = sqlx::query_as::<_, MonthlyBurnInResponse>(
            r#"
WITH total_budget AS (
    SELECT COALESCE(SUM(bc.budgeted_value), 0)::int AS value
    FROM budget_category bc
    WHERE bc.user_id = $2
),
spent_budget AS (
    SELECT COALESCE(SUM(t.amount), 0)::int AS value
    FROM transaction t
    JOIN  category c       ON t.category_id = c.id
    CROSS JOIN budget_period bp
    WHERE bp.id            = $1
      AND bp.user_id       = $2
      AND t.user_id        = $2
      AND c.category_type  = 'Outgoing'
      AND t.occurred_at   >= bp.start_date
      AND t.occurred_at   <= bp.end_date
)
SELECT
    total_budget.value                      AS total_budget,
    spent_budget.value                      AS spent_budget,
    (CURRENT_DATE - bp.start_date)::int     AS current_day,
    (bp.end_date  - bp.start_date)::int     AS days_in_period
FROM budget_period bp
CROSS JOIN total_budget
CROSS JOIN spent_budget
WHERE bp.id = $1 AND bp.user_id = $2
            "#,
        )
        .bind(budget_period_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(row)
    }

    pub async fn month_progress(&self, budget_period_id: &Uuid, user_id: &Uuid) -> Result<MonthProgressResponse, AppError> {
        #[derive(sqlx::FromRow)]
        struct MonthProgressRow {
            current_date: NaiveDate,
            days_in_period: i32,
            remaining_days: i32,
            days_passed_percentage: i32,
        }

        let row = sqlx::query_as::<_, MonthProgressRow>(
            r#"
SELECT
    CURRENT_DATE                                                                                                          AS current_date,
    GREATEST(1, (bp.end_date - bp.start_date)::int)                                                                        AS days_in_period,
    GREATEST(0, (bp.end_date - CURRENT_DATE)::int)                                                                        AS remaining_days,
    (LEAST((bp.end_date - bp.start_date)::int, GREATEST(0, (CURRENT_DATE - bp.start_date)::int)) * 100
        / GREATEST(1, (bp.end_date - bp.start_date)::int))::int                                                            AS days_passed_percentage
FROM budget_period bp
WHERE bp.id = $1 AND bp.user_id = $2
            "#,
        )
        .bind(budget_period_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(MonthProgressResponse {
            current_date: row.current_date,
            days_in_period: row.days_in_period as u32,
            remaining_days: row.remaining_days as u32,
            days_passed_percentage: row.days_passed_percentage as u32,
        })
    }

    pub async fn get_total_assets(&self, user_id: &Uuid) -> Result<TotalAssetsResponse, AppError> {
        #[derive(sqlx::FromRow)]
        struct TotalAssetsRow {
            total_assets: i32,
        }

        let row = sqlx::query_as::<_, TotalAssetsRow>(
            r#"
WITH account_initial_balances AS (
    SELECT
        SUM(balance) as balance_total
    FROM account
    WHERE user_id = $1
)
SELECT
    (SUM(
            CASE
                WHEN c.category_type = 'Incoming'                              THEN  t.amount
                WHEN c.category_type = 'Outgoing'                              THEN -t.amount
                WHEN c.category_type = 'Transfer' AND t.from_account_id = a.id THEN -t.amount
                WHEN c.category_type = 'Transfer' AND t.to_account_id   = a.id THEN  t.amount
                ELSE 0
                END
    ) + aib.balance_total)::int as total_assets
FROM transaction t
JOIN account a ON a.id = t.from_account_id OR a.id = t.to_account_id
JOIN category c ON c.id = t.category_id
CROSS JOIN account_initial_balances aib
WHERE t.user_id = $1 AND a.account_type <> 'Allowance'
GROUP BY aib.balance_total
"#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(TotalAssetsResponse {
            total_assets: row.total_assets,
        })
    }
}
