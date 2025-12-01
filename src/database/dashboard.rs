use crate::error::app_error::AppError;
use crate::models::dashboard::{
    BudgetPerDayResponse, MonthlyBurnInResponse, SpentPerCategoryResponse,
};
use deadpool_postgres::Client;
use tracing::error;

pub async fn balance_per_day(client: &Client) -> Result<Vec<BudgetPerDayResponse>, AppError> {
    let rows = client
        .query(
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
            &[],
        )
        .await?;

    Ok(rows
        .iter()
        .map(|row| {
            let balance = row.try_get::<_, i32>("balance").unwrap_or_else(|err| {
                error!("Error: {:?}", err);
                0
            });

            BudgetPerDayResponse {
                account_name: row.get("account_name"),
                date: row.get("date"),
                balance,
            }
        })
        .collect())
}

pub async fn spent_per_category(
    client: &Client,
) -> Result<Vec<SpentPerCategoryResponse>, AppError> {
    let rows = client
        .query(
            r#"
            SELECT
                c.name as category_name,
                bc.budgeted_value,
                CAST(SUM(t.amount) AS INTEGER) as amount_spent
            FROM category c
            JOIN budget_category bc ON c.id = bc.category_id
            JOIN transaction t ON c.id = t.category_id
            JOIN budget b ON 1=1
            WHERE CASE
                WHEN EXTRACT(MONTH FROM now()) > 1
                  THEN t.occurred_at > MAKE_DATE(CAST(EXTRACT(YEAR FROM now()) AS INTEGER), CAST(EXTRACT(MONTH FROM now()) - 1 AS INTEGER), b.start_day)
                ELSE
                  t.occurred_at > MAKE_DATE(CAST(EXTRACT(YEAR FROM now()) - 1 AS INTEGER), 12, b.start_day)
                END
            GROUP BY c.name, bc.budgeted_value
            "#,
            &[],
        )
        .await?;

    Ok(rows
        .iter()
        .map(|row| SpentPerCategoryResponse {
            category_name: row.get("category_name"),
            budgeted_value: row.get("budgeted_value"),
            amount_spent: row.get("amount_spent"),
        })
        .collect())
}

pub async fn monthly_burn_in(client: &Client) -> Result<Vec<MonthlyBurnInResponse>, AppError> {
    let rows = client.query(
        r#"
        WITH total_budget AS (
        SELECT
            CAST(SUM(bc.budgeted_value) AS INTEGER) as value
        FROM budget_category bc
        ), spent_budget AS (
        SELECT CAST(SUM(t.amount) AS INTEGER) as value
        FROM budget_category bc
            JOIN transaction t ON bc.category_id = t.category_id
            JOIN budget b ON 1 = 1
        WHERE CASE
            WHEN EXTRACT(MONTH FROM now()) > 1
                THEN t.occurred_at > MAKE_DATE(CAST(EXTRACT(YEAR FROM now()) AS INTEGER), CAST(EXTRACT(MONTH FROM now()) - 1 AS INTEGER), b.start_day)
                ELSE t.occurred_at > MAKE_DATE(CAST(EXTRACT(YEAR FROM now()) - 1 AS INTEGER), 12, b.start_day)
            END
        )
        SELECT
            total_budget.value as total_budget,
            spent_budget.value as spent_budget,
            MAKE_DATE(CAST(EXTRACT(YEAR FROM now()) AS INTEGER), CAST(EXTRACT(MONTH FROM now()) AS INTEGER), CAST(EXTRACT(DAY FROM now()) AS INTEGER)) - CASE WHEN EXTRACT(MONTH FROM now()) > 1
                THEN MAKE_DATE(CAST(EXTRACT(YEAR FROM now()) AS INTEGER), CAST(EXTRACT(MONTH FROM now()) - 1 AS INTEGER), b.start_day)
                ELSE MAKE_DATE(CAST(EXTRACT(YEAR FROM now()) - 1 AS INTEGER), 12, b.start_day)
                END as current_day,
            MAKE_DATE(CAST(EXTRACT(YEAR FROM now()) AS INTEGER), CAST(EXTRACT(MONTH FROM now()) AS INTEGER), b.start_day) - CASE WHEN EXTRACT(MONTH FROM now()) > 1
                THEN MAKE_DATE(CAST(EXTRACT(YEAR FROM now()) AS INTEGER), CAST(EXTRACT(MONTH FROM now()) - 1 AS INTEGER), b.start_day)
                ELSE MAKE_DATE(CAST(EXTRACT(YEAR FROM now()) - 1 AS INTEGER), 12, b.start_day)
            END as days_in_period
        FROM total_budget
        JOIN spent_budget ON 1=1
        JOIN budget b ON 1=1
        "#, &[]
    ).await?;

    Ok(rows
        .iter()
        .map(|row| MonthlyBurnInResponse {
            total_budget: row.get("total_budget"),
            spent_budget: row.get("spent_budget"),
            current_day: row.get("current_day"),
            days_in_period: row.get("days_in_period"),
        })
        .collect())
}
