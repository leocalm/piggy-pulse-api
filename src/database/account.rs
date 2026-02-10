use crate::database::postgres_repository::{PostgresRepository, is_unique_violation};
use crate::error::app_error::AppError;
use crate::models::account::{Account, AccountBalancePerDay, AccountRequest, AccountType, AccountWithMetrics};
use crate::models::currency::Currency;
use crate::models::pagination::CursorParams;
use chrono::{DateTime, Utc};
use uuid::Uuid;

// Intermediate struct for sqlx query results with JOINed currency data
#[derive(Debug, sqlx::FromRow)]
struct AccountRow {
    id: Uuid,
    user_id: Uuid,
    name: String,
    color: String,
    icon: String,
    account_type: String,
    balance: i64,
    created_at: DateTime<Utc>,
    spend_limit: Option<i32>,
    currency_id: Uuid,
    currency_name: String,
    currency_symbol: String,
    currency_code: String,
    currency_decimal_places: i32,
    currency_created_at: DateTime<Utc>,
}

impl From<AccountRow> for Account {
    fn from(row: AccountRow) -> Self {
        Account {
            id: row.id,
            user_id: row.user_id,
            name: row.name,
            color: row.color,
            icon: row.icon,
            account_type: account_type_from_db(&row.account_type),
            currency: Currency {
                id: row.currency_id,
                name: row.currency_name,
                symbol: row.currency_symbol,
                currency: row.currency_code,
                decimal_places: row.currency_decimal_places,
                created_at: row.currency_created_at,
            },
            balance: row.balance,
            created_at: row.created_at,
            spend_limit: row.spend_limit,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct AccountMetricsRow {
    id: Uuid,
    user_id: Uuid,
    name: String,
    color: String,
    icon: String,
    account_type: String,
    balance: i64,
    created_at: DateTime<Utc>,
    spend_limit: Option<i32>,
    currency_id: Uuid,
    currency_name: String,
    currency_symbol: String,
    currency_code: String,
    currency_decimal_places: i32,
    currency_created_at: DateTime<Utc>,
    current_balance: i64,
    balance_change_this_period: i64,
    transaction_count: i64,
}

impl From<AccountMetricsRow> for AccountWithMetrics {
    fn from(row: AccountMetricsRow) -> Self {
        AccountWithMetrics {
            account: Account {
                id: row.id,
                user_id: row.user_id,
                name: row.name,
                color: row.color,
                icon: row.icon,
                account_type: account_type_from_db(&row.account_type),
                currency: Currency {
                    id: row.currency_id,
                    name: row.currency_name,
                    symbol: row.currency_symbol,
                    currency: row.currency_code,
                    decimal_places: row.currency_decimal_places,
                    created_at: row.currency_created_at,
                },
                balance: row.balance,
                created_at: row.created_at,
                spend_limit: row.spend_limit,
            },
            current_balance: row.current_balance,
            balance_change_this_period: row.balance_change_this_period,
            transaction_count: row.transaction_count,
        }
    }
}

impl PostgresRepository {
    pub async fn create_account(&self, request: &AccountRequest, user_id: &Uuid) -> Result<Account, AppError> {
        let name_exists: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS (
                SELECT 1
                FROM account
                WHERE user_id = $1 AND name = $2
            )
            "#,
        )
        .bind(user_id)
        .bind(&request.name)
        .fetch_one(&self.pool)
        .await?;

        if name_exists {
            return Err(AppError::BadRequest("Account name already exists".to_string()));
        }

        let currency = self
            .get_currency_by_code(&request.currency, user_id)
            .await?
            .ok_or_else(|| AppError::CurrencyDoesNotExist(request.currency.clone()))?;

        let account_type_str = request.account_type_to_db();

        #[derive(sqlx::FromRow)]
        struct CreateAccountRow {
            id: Uuid,
            user_id: Uuid,
            name: String,
            color: String,
            icon: String,
            account_type: String,
            balance: i64,
            created_at: DateTime<Utc>,
            spend_limit: Option<i32>,
        }

        let row = sqlx::query_as::<_, CreateAccountRow>(
            r#"
            INSERT INTO account (user_id, name, color, icon, account_type, currency_id, balance, spend_limit)
            VALUES ($1, $2, $3, $4, $5::text::account_type, $6, $7, $8)
            RETURNING
                id,
                user_id,
                name,
                color,
                icon,
                account_type::text as account_type,
                balance,
                currency_id,
                created_at,
                spend_limit
            "#,
        )
        .bind(user_id)
        .bind(&request.name)
        .bind(&request.color)
        .bind(&request.icon)
        .bind(&account_type_str)
        .bind(currency.id)
        .bind(request.balance)
        .bind(request.spend_limit)
        .fetch_one(&self.pool)
        .await;

        let row = match row {
            Ok(row) => row,
            Err(err) if is_unique_violation(&err) => {
                return Err(AppError::BadRequest("Account name already exists".to_string()));
            }
            Err(err) => return Err(err.into()),
        };

        Ok(Account {
            id: row.id,
            user_id: row.user_id,
            name: row.name,
            color: row.color,
            icon: row.icon,
            account_type: account_type_from_db(&row.account_type),
            currency,
            balance: row.balance,
            created_at: row.created_at,
            spend_limit: row.spend_limit,
        })
    }

    pub async fn get_account_by_id(&self, id: &Uuid, user_id: &Uuid) -> Result<Option<Account>, AppError> {
        let row = sqlx::query_as::<_, AccountRow>(
            r#"
            SELECT
                a.id,
                a.user_id,
                a.name,
                a.color,
                a.icon,
                a.account_type::text as account_type,
                a.balance,
                a.created_at,
                a.spend_limit,
                c.id as currency_id,
                c.name as currency_name,
                c.symbol as currency_symbol,
                c.currency as currency_code,
                c.decimal_places as currency_decimal_places,
                c.created_at as currency_created_at
            FROM account a
            JOIN currency c ON c.id = a.currency_id
            WHERE a.id = $1 AND a.user_id = $2
            "#,
        )
        .bind(id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Account::from))
    }

    pub async fn list_accounts(&self, params: &CursorParams, budget_period_id: &Uuid, user_id: &Uuid) -> Result<Vec<AccountWithMetrics>, AppError> {
        let rows = if let Some(cursor) = params.cursor {
            sqlx::query_as::<_, AccountMetricsRow>(
                r#"
                WITH period AS (
                    SELECT start_date, end_date
                    FROM budget_period
                    WHERE id = $2 AND user_id = $3
                )
                SELECT
                    a.id,
                    a.user_id,
                    a.name,
                    a.color,
                    a.icon,
                    a.account_type::text as account_type,
                    a.balance,
                    a.created_at,
                    a.spend_limit,
                    c.id as currency_id,
                    c.name as currency_name,
                    c.symbol as currency_symbol,
                    c.currency as currency_code,
                    c.decimal_places as currency_decimal_places,
                    c.created_at as currency_created_at,
                    (a.balance + COALESCE(SUM(
                        CASE
                            WHEN cat.category_type = 'Incoming'                              THEN  t.amount::bigint
                            WHEN cat.category_type = 'Outgoing'                              THEN -t.amount::bigint
                            WHEN cat.category_type = 'Transfer' AND t.from_account_id = a.id THEN -t.amount::bigint
                            WHEN cat.category_type = 'Transfer' AND t.to_account_id   = a.id THEN  t.amount::bigint
                            ELSE 0
                        END
                    ), 0))::bigint AS current_balance,
                    COALESCE(SUM(
                        CASE
                            WHEN p.start_date IS NOT NULL
                             AND t.occurred_at >= p.start_date
                             AND t.occurred_at <= p.end_date THEN
                                CASE
                                    WHEN cat.category_type = 'Incoming'                              THEN  t.amount::bigint
                                    WHEN cat.category_type = 'Outgoing'                              THEN -t.amount::bigint
                                    WHEN cat.category_type = 'Transfer' AND t.from_account_id = a.id THEN -t.amount::bigint
                                    WHEN cat.category_type = 'Transfer' AND t.to_account_id   = a.id THEN  t.amount::bigint
                                    ELSE 0
                                END
                            ELSE 0
                        END
                    ), 0)::bigint AS balance_change_this_period,
                    COALESCE(SUM(
                        CASE
                            WHEN t.from_account_id = a.id THEN 1
                            ELSE 0
                        END
                    ), 0)::bigint AS transaction_count
                FROM account a
                JOIN currency c ON c.id = a.currency_id
                LEFT JOIN period p ON true
                LEFT JOIN transaction t ON (t.from_account_id = a.id OR t.to_account_id = a.id) AND t.user_id = $3
                LEFT JOIN category cat ON t.category_id = cat.id
                WHERE (a.created_at, a.id) < (
                    SELECT created_at, id FROM account WHERE id = $1
                ) AND a.user_id = $3
                GROUP BY
                    a.id,
                    a.user_id,
                    a.name,
                    a.color,
                    a.icon,
                    a.account_type,
                    a.balance,
                    a.created_at,
                    a.spend_limit,
                    c.id,
                    c.name,
                    c.symbol,
                    c.currency,
                    c.decimal_places,
                    c.created_at
                ORDER BY a.created_at DESC, a.id DESC
                LIMIT $4
                "#,
            )
            .bind(cursor)
            .bind(budget_period_id)
            .bind(user_id)
            .bind(params.fetch_limit())
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, AccountMetricsRow>(
                r#"
                WITH period AS (
                    SELECT start_date, end_date
                    FROM budget_period
                    WHERE id = $1 AND user_id = $2
                )
                SELECT
                    a.id,
                    a.user_id,
                    a.name,
                    a.color,
                    a.icon,
                    a.account_type::text as account_type,
                    a.balance,
                    a.created_at,
                    a.spend_limit,
                    c.id as currency_id,
                    c.name as currency_name,
                    c.symbol as currency_symbol,
                    c.currency as currency_code,
                    c.decimal_places as currency_decimal_places,
                    c.created_at as currency_created_at,
                    (a.balance + COALESCE(SUM(
                        CASE
                            WHEN cat.category_type = 'Incoming'                              THEN  t.amount::bigint
                            WHEN cat.category_type = 'Outgoing'                              THEN -t.amount::bigint
                            WHEN cat.category_type = 'Transfer' AND t.from_account_id = a.id THEN -t.amount::bigint
                            WHEN cat.category_type = 'Transfer' AND t.to_account_id   = a.id THEN  t.amount::bigint
                            ELSE 0
                        END
                    ), 0))::bigint AS current_balance,
                    COALESCE(SUM(
                        CASE
                            WHEN p.start_date IS NOT NULL
                             AND t.occurred_at >= p.start_date
                             AND t.occurred_at <= p.end_date THEN
                                CASE
                                    WHEN cat.category_type = 'Incoming'                              THEN  t.amount::bigint
                                    WHEN cat.category_type = 'Outgoing'                              THEN -t.amount::bigint
                                    WHEN cat.category_type = 'Transfer' AND t.from_account_id = a.id THEN -t.amount::bigint
                                    WHEN cat.category_type = 'Transfer' AND t.to_account_id   = a.id THEN  t.amount::bigint
                                    ELSE 0
                                END
                            ELSE 0
                        END
                    ), 0)::bigint AS balance_change_this_period,
                    COALESCE(SUM(
                        CASE
                            WHEN t.from_account_id = a.id THEN 1
                            ELSE 0
                        END
                    ), 0)::bigint AS transaction_count
                FROM account a
                JOIN currency c ON c.id = a.currency_id
                LEFT JOIN period p ON true
                LEFT JOIN transaction t ON (t.from_account_id = a.id OR t.to_account_id = a.id) AND t.user_id = $2
                LEFT JOIN category cat ON t.category_id = cat.id
                WHERE a.user_id = $2
                GROUP BY
                    a.id,
                    a.user_id,
                    a.name,
                    a.color,
                    a.icon,
                    a.account_type,
                    a.balance,
                    a.created_at,
                    a.spend_limit,
                    c.id,
                    c.name,
                    c.symbol,
                    c.currency,
                    c.decimal_places,
                    c.created_at
                ORDER BY a.created_at DESC, a.id DESC
                LIMIT $3
                "#,
            )
            .bind(budget_period_id)
            .bind(user_id)
            .bind(params.fetch_limit())
            .fetch_all(&self.pool)
            .await?
        };

        Ok(rows.into_iter().map(AccountWithMetrics::from).collect())
    }

    pub async fn list_account_balance_per_day(
        &self,
        account_ids: &[Uuid],
        budget_period_id: &Uuid,
        user_id: &Uuid,
    ) -> Result<Vec<AccountBalancePerDay>, AppError> {
        if account_ids.is_empty() {
            return Ok(Vec::new());
        }

        #[derive(sqlx::FromRow)]
        struct BalancePerDayRow {
            account_id: Uuid,
            account_name: String,
            date: String,
            balance: i64,
        }

        let rows = sqlx::query_as::<_, BalancePerDayRow>(
            r#"
WITH period AS (
    SELECT start_date, end_date
    FROM budget_period
    WHERE id = $2 AND user_id = $3
),
days AS (
    SELECT d::date AS day
    FROM generate_series(
        (SELECT start_date FROM period),
        (SELECT end_date FROM period),
        '1 day'
    ) AS d
),
base_balances AS (
    SELECT
        a.id,
        a.balance + COALESCE(SUM(
            CASE
                WHEN c.category_type = 'Incoming'                              THEN  t.amount::bigint
                WHEN c.category_type = 'Outgoing'                              THEN -t.amount::bigint
                WHEN c.category_type = 'Transfer' AND t.from_account_id = a.id THEN -t.amount::bigint
                WHEN c.category_type = 'Transfer' AND t.to_account_id   = a.id THEN  t.amount::bigint
                ELSE 0
            END
        ), 0) AS base_balance
    FROM account a
    LEFT JOIN transaction t ON (t.from_account_id = a.id OR t.to_account_id = a.id)
                            AND t.occurred_at < (SELECT start_date FROM period)
                            AND t.user_id = $3
    LEFT JOIN category c    ON t.category_id = c.id
    WHERE a.user_id = $3 AND a.id = ANY($1)
    GROUP BY a.id, a.balance
),
daily_totals AS (
    SELECT
        a.id AS account_id,
        t.occurred_at::date AS occurred_date,
        SUM(
            CASE
                WHEN c.category_type = 'Incoming'                              THEN  t.amount::bigint
                WHEN c.category_type = 'Outgoing'                              THEN -t.amount::bigint
                WHEN c.category_type = 'Transfer' AND t.from_account_id = a.id THEN -t.amount::bigint
                WHEN c.category_type = 'Transfer' AND t.to_account_id   = a.id THEN  t.amount::bigint
                ELSE 0
            END
        ) AS daily_amount
    FROM account a
    JOIN transaction t  ON t.from_account_id = a.id OR t.to_account_id = a.id
    JOIN category   c   ON t.category_id = c.id
    CROSS JOIN period
    WHERE a.user_id = $3
      AND t.user_id = $3
      AND a.id = ANY($1)
      AND t.occurred_at >= period.start_date
      AND t.occurred_at <= period.end_date
    GROUP BY a.id, t.occurred_at::date
)
SELECT
    a.id                           AS account_id,
    a.name                         AS account_name,
    to_char(d.day, 'YYYY-MM-DD')   AS date,
    (bb.base_balance + SUM(COALESCE(dt.daily_amount, 0)) OVER (
        PARTITION BY a.id
        ORDER BY d.day
        ROWS UNBOUNDED PRECEDING
    ))::bigint                     AS balance
FROM account a
JOIN  base_balances bb ON bb.id = a.id
CROSS JOIN days d
LEFT JOIN daily_totals dt ON dt.account_id = a.id AND dt.occurred_date = d.day
WHERE a.user_id = $3 AND a.id = ANY($1)
ORDER BY a.id, d.day
            "#,
        )
        .bind(account_ids)
        .bind(budget_period_id)
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| AccountBalancePerDay {
                account_id: row.account_id,
                account_name: row.account_name,
                date: row.date,
                balance: row.balance,
            })
            .collect())
    }

    pub async fn delete_account(&self, id: &Uuid, user_id: &Uuid) -> Result<(), AppError> {
        sqlx::query("DELETE FROM account WHERE id = $1 AND user_id = $2")
            .bind(id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn get_account_options(&self, user_id: &Uuid) -> Result<Vec<(Uuid, String, String)>, AppError> {
        #[derive(sqlx::FromRow)]
        struct AccountOptionRow {
            id: Uuid,
            name: String,
            icon: String,
        }

        let rows = sqlx::query_as::<_, AccountOptionRow>(
            r#"
            SELECT id, name, icon
            FROM account
            WHERE user_id = $1
            ORDER BY name ASC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|row| (row.id, row.name, row.icon)).collect())
    }

    pub async fn get_accounts_summary(&self, user_id: &Uuid) -> Result<(i64, i64, i64), AppError> {
        #[derive(sqlx::FromRow)]
        struct SummaryRow {
            total_assets: i64,
            total_liabilities: i64,
        }

        let row = sqlx::query_as::<_, SummaryRow>(
            r#"
            SELECT
                COALESCE(SUM(CASE
                    WHEN account_type::text IN ('Checking', 'Savings', 'Wallet') THEN balance
                    ELSE 0
                END), 0)::bigint AS total_assets,
                COALESCE(SUM(CASE
                    WHEN account_type::text = 'CreditCard' THEN balance
                    ELSE 0
                END), 0)::bigint AS total_liabilities
            FROM account
            WHERE user_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        let total_net_worth = row.total_assets - row.total_liabilities;
        Ok((total_net_worth, row.total_assets, row.total_liabilities))
    }

    pub async fn update_account(&self, id: &Uuid, request: &AccountRequest, user_id: &Uuid) -> Result<Account, AppError> {
        let name_exists: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS (
                SELECT 1
                FROM account
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
            return Err(AppError::BadRequest("Account name already exists".to_string()));
        }

        let currency = self
            .get_currency_by_code(&request.currency, user_id)
            .await?
            .ok_or_else(|| AppError::CurrencyDoesNotExist(request.currency.clone()))?;

        let account_type_str = request.account_type_to_db();

        #[derive(sqlx::FromRow)]
        struct UpdateAccountRow {
            id: Uuid,
            user_id: Uuid,
            name: String,
            color: String,
            icon: String,
            account_type: String,
            balance: i64,
            created_at: DateTime<Utc>,
            spend_limit: Option<i32>,
        }

        let row = sqlx::query_as::<_, UpdateAccountRow>(
            r#"
            UPDATE account
            SET name = $1, color = $2, icon = $3, account_type = $4::text::account_type, currency_id = $5, balance = $6
            WHERE id = $7 and user_id = $8
            RETURNING
                id,
                user_id,
                name,
                color,
                icon,
                account_type::text as account_type,
                balance,
                currency_id,
                created_at,
                spend_limit
            "#,
        )
        .bind(&request.name)
        .bind(&request.color)
        .bind(&request.icon)
        .bind(&account_type_str)
        .bind(currency.id)
        .bind(request.balance)
        .bind(id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await;

        let row = match row {
            Ok(row) => row,
            Err(err) if is_unique_violation(&err) => {
                return Err(AppError::BadRequest("Account name already exists".to_string()));
            }
            Err(err) => return Err(err.into()),
        };

        Ok(Account {
            id: row.id,
            user_id: row.user_id,
            name: row.name,
            color: row.color,
            icon: row.icon,
            account_type: account_type_from_db(&row.account_type),
            currency,
            balance: row.balance,
            created_at: row.created_at,
            spend_limit: row.spend_limit,
        })
    }
}

pub fn account_type_from_db<T: AsRef<str>>(value: T) -> AccountType {
    match value.as_ref() {
        "Checking" => AccountType::Checking,
        "Savings" => AccountType::Savings,
        "CreditCard" => AccountType::CreditCard,
        "Wallet" => AccountType::Wallet,
        "Allowance" => AccountType::Allowance,
        other => panic!("Unknown account type: {}", other),
    }
}

trait AccountRequestDbExt {
    fn account_type_to_db(&self) -> String;
}

impl AccountRequestDbExt for AccountRequest {
    fn account_type_to_db(&self) -> String {
        match self.account_type {
            AccountType::Checking => "Checking".to_string(),
            AccountType::Savings => "Savings".to_string(),
            AccountType::CreditCard => "CreditCard".to_string(),
            AccountType::Wallet => "Wallet".to_string(),
            AccountType::Allowance => "Allowance".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_account_type_from_db_all_types() {
        assert!(matches!(account_type_from_db("Checking"), AccountType::Checking));
        assert!(matches!(account_type_from_db("Savings"), AccountType::Savings));
        assert!(matches!(account_type_from_db("CreditCard"), AccountType::CreditCard));
        assert!(matches!(account_type_from_db("Wallet"), AccountType::Wallet));
        assert!(matches!(account_type_from_db("Allowance"), AccountType::Allowance));
    }

    #[test]
    #[should_panic(expected = "Unknown account type")]
    fn test_account_type_from_db_invalid() {
        account_type_from_db("InvalidType");
    }

    #[test]
    fn test_account_type_to_db() {
        let request = AccountRequest {
            name: "Test".to_string(),
            color: "#000000".to_string(),
            icon: "icon".to_string(),
            account_type: AccountType::Checking,
            currency: "USD".to_string(),
            balance: 0,
            spend_limit: None,
        };
        assert_eq!(request.account_type_to_db(), "Checking");

        let request_savings = AccountRequest {
            name: "Test".to_string(),
            color: "#000000".to_string(),
            icon: "icon".to_string(),
            account_type: AccountType::Savings,
            currency: "USD".to_string(),
            balance: 0,
            spend_limit: None,
        };
        assert_eq!(request_savings.account_type_to_db(), "Savings");

        let request_credit = AccountRequest {
            name: "Test".to_string(),
            color: "#000000".to_string(),
            icon: "icon".to_string(),
            account_type: AccountType::CreditCard,
            currency: "USD".to_string(),
            balance: 0,
            spend_limit: None,
        };
        assert_eq!(request_credit.account_type_to_db(), "CreditCard");
    }
}
