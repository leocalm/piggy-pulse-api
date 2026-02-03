use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::models::dashboard::{BudgetPerDayResponse, MonthlyBurnInResponse, SpentPerCategoryResponse};

#[async_trait::async_trait]
pub trait DashboardRepository {
    async fn balance_per_day(&self) -> Result<Vec<BudgetPerDayResponse>, AppError>;
    async fn spent_per_category(&self) -> Result<Vec<SpentPerCategoryResponse>, AppError>;
    async fn monthly_burn_in(&self) -> Result<MonthlyBurnInResponse, AppError>;
}

#[async_trait::async_trait]
impl DashboardRepository for PostgresRepository {
    async fn balance_per_day(&self) -> Result<Vec<BudgetPerDayResponse>, AppError> {
        #[derive(sqlx::FromRow)]
        struct BalancePerDayRow {
            account_name: String,
            date: String,
            balance: i32,
        }

        let rows = sqlx::query_as::<_, BalancePerDayRow>(
            r#"
            SELECT
                a.name as account_name,
                to_char(t.occurred_at, 'YYYY-MM-DD') as date,
                CAST(SUM(SUM(CASE c.category_type WHEN 'Incoming' THEN t.amount WHEN 'Outgoing' THEN -t.amount END)) OVER (ORDER by t.occurred_at) + a.balance AS INTEGER) as balance
            FROM transaction t
                JOIN category c ON t.category_id = c.id
                JOIN account a ON t.from_account_id = a.id
                JOIN budget b ON 1=1
            WHERE CASE
                WHEN EXTRACT(MONTH FROM now()) > 1
                    THEN t.occurred_at > MAKE_DATE(CAST(EXTRACT(YEAR FROM now()) AS INTEGER), CAST(EXTRACT(MONTH FROM now()) - 1 AS INTEGER), b.start_day)
                ELSE
                    t.occurred_at > MAKE_DATE(CAST(EXTRACT(YEAR FROM now()) - 1 AS INTEGER), 12, b.start_day)
                END
            GROUP BY t.occurred_at, a.name, a.balance
            ORDER BY t.occurred_at
            "#,
        )
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

    async fn spent_per_category(&self) -> Result<Vec<SpentPerCategoryResponse>, AppError> {
        #[derive(sqlx::FromRow)]
        struct SpentPerCategoryRow {
            category_name: String,
            budgeted_value: i32,
            amount_spent: i32,
        }

        let rows = sqlx::query_as::<_, SpentPerCategoryRow>(
            r#"
WITH budget_settings AS (
    SELECT b.start_day
    FROM budget b
), cutoff AS (
    SELECT CASE
               WHEN EXTRACT(DAY from now())::int >= s.start_day THEN
                   MAKE_DATE(EXTRACT(YEAR FROM now())::int,
                             (EXTRACT(MONTH FROM now()))::int,
                             s.start_day)
               WHEN EXTRACT(MONTH FROM now()) > 1 THEN
                   MAKE_DATE(EXTRACT(YEAR FROM now())::int,
                             (EXTRACT(MONTH FROM now()) - 1)::int,
                             s.start_day)
               ELSE
                   MAKE_DATE((EXTRACT(YEAR FROM now()) - 1)::int, 12, s.start_day)
               END AS start_date
    FROM budget_settings s
), month_transactions AS (
    SELECT t.category_id, t.amount
    FROM transaction t
             CROSS JOIN cutoff c
    WHERE t.occurred_at > c.start_date
)
SELECT c.name AS category_name,
       bc.budgeted_value,
       COALESCE(SUM(mt.amount), 0)::int AS amount_spent
FROM category c
LEFT JOIN month_transactions mt
    ON c.id = mt.category_id
JOIN budget_category bc
    ON bc.category_id = c.id
GROUP BY category_name, budgeted_value
            "#,
        )
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

    async fn monthly_burn_in(&self) -> Result<MonthlyBurnInResponse, AppError> {
        let row = sqlx::query_as::<_, MonthlyBurnInResponse>(
            r#"
WITH budget_settings AS (
    SELECT b.start_day
    FROM budget b
), cutoff AS (
    SELECT CASE
        WHEN EXTRACT(DAY from now())::int >= s.start_day THEN
            MAKE_DATE(EXTRACT(YEAR FROM now())::int,
                      (EXTRACT(MONTH FROM now()))::int,
                      s.start_day)
        WHEN EXTRACT(MONTH FROM now()) > 1 THEN
                   MAKE_DATE(EXTRACT(YEAR FROM now())::int,
                             (EXTRACT(MONTH FROM now()) - 1)::int,
                             s.start_day)
               ELSE
                   MAKE_DATE((EXTRACT(YEAR FROM now()) - 1)::int, 12, s.start_day)
               END AS start_date,
        CASE
            WHEN EXTRACT(DAY from now())::int < s.start_day THEN
                MAKE_DATE(EXTRACT(YEAR FROM now())::int,
                          (EXTRACT(MONTH FROM now()))::int,
                          s.start_day)
            WHEN EXTRACT(MONTH FROM now())::int = 12 THEN
                MAKE_DATE(EXTRACT(YEAR FROM now())::int + 1,
                          1,
                          s.start_day)
            ELSE
                MAKE_DATE(EXTRACT(YEAR FROM now())::int,
                          (EXTRACT(MONTH FROM now()) + 1)::int,
                          s.start_day)
        END AS end_date
    FROM budget_settings s
), total_budget AS (
    SELECT CAST(SUM(bc.budgeted_value) AS INTEGER) as value
    FROM budget_category bc
), spent_budget AS (
    SELECT CAST(SUM(t.amount) AS INTEGER) as value
    FROM budget_category bc
             JOIN transaction t ON bc.category_id = t.category_id
             CROSS JOIN cutoff
    WHERE t.occurred_at > cutoff.start_date
)
SELECT
    total_budget.value as total_budget,
    COALESCE(spent_budget.value, 0)::int as spent_budget,
    MAKE_DATE(CAST(EXTRACT(YEAR FROM now()) AS INTEGER), CAST(EXTRACT(MONTH FROM now()) AS INTEGER), CAST(EXTRACT(DAY FROM now()) AS INTEGER)) - cutoff.start_date as current_day,
    cutoff.end_date - cutoff.start_date as days_in_period
FROM total_budget
CROSS JOIN spent_budget
CROSS JOIN cutoff
        "#,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(row)
    }
}
