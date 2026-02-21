use crate::database::postgres_repository::{PostgresRepository, is_unique_violation};
use crate::error::app_error::AppError;
use crate::models::account::{
    Account, AccountBalanceHistoryPoint, AccountBalancePerDay, AccountDetailResponse, AccountManagementResponse, AccountRequest, AccountTransactionResponse,
    AccountType, AccountUpdateRequest, AccountWithMetrics,
};
use crate::models::currency::{Currency, CurrencyResponse, SymbolPosition};
use crate::models::pagination::CursorParams;
use chrono::NaiveDate;
use uuid::Uuid;

// Intermediate struct for sqlx query results with JOINed currency data
#[derive(Debug, sqlx::FromRow)]
struct AccountRow {
    id: Uuid,
    name: String,
    color: String,
    icon: String,
    account_type: String,
    balance: i64,
    spend_limit: Option<i32>,
    is_archived: bool,
    next_transfer_amount: Option<i64>,
    currency_id: Uuid,
    currency_name: String,
    currency_symbol: String,
    currency_code: String,
    currency_decimal_places: i32,
    currency_symbol_position: SymbolPosition,
}

impl From<AccountRow> for Account {
    fn from(row: AccountRow) -> Self {
        Account {
            id: row.id,
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
                symbol_position: row.currency_symbol_position,
            },
            balance: row.balance,
            spend_limit: row.spend_limit,
            is_archived: row.is_archived,
            next_transfer_amount: row.next_transfer_amount,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct AccountMetricsRow {
    id: Uuid,
    name: String,
    color: String,
    icon: String,
    account_type: String,
    balance: i64,
    spend_limit: Option<i32>,
    is_archived: bool,
    next_transfer_amount: Option<i64>,
    currency_id: Uuid,
    currency_name: String,
    currency_symbol: String,
    currency_code: String,
    currency_decimal_places: i32,
    currency_symbol_position: SymbolPosition,
    current_balance: i64,
    balance_change_this_period: i64,
    transaction_count: i64,
}

impl From<AccountMetricsRow> for AccountWithMetrics {
    fn from(row: AccountMetricsRow) -> Self {
        AccountWithMetrics {
            account: Account {
                id: row.id,
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
                    symbol_position: row.currency_symbol_position,
                },
                balance: row.balance,
                spend_limit: row.spend_limit,
                is_archived: row.is_archived,
                next_transfer_amount: row.next_transfer_amount,
            },
            current_balance: row.current_balance,
            balance_change_this_period: row.balance_change_this_period,
            transaction_count: row.transaction_count,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct AccountManagementRow {
    id: Uuid,
    name: String,
    color: String,
    icon: String,
    account_type: String,
    balance: i64,
    spend_limit: Option<i32>,
    is_archived: bool,
    next_transfer_amount: Option<i64>,
    currency_id: Uuid,
    currency_name: String,
    currency_symbol: String,
    currency_code: String,
    currency_decimal_places: i32,
    currency_symbol_position: SymbolPosition,
    transaction_count: i64,
    can_delete: bool,
    can_adjust_balance: bool,
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

        let default_currency_id: Option<Uuid> = sqlx::query_scalar(
            r#"
            SELECT default_currency_id
            FROM settings
            WHERE user_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?
        .flatten();

        let currency_id = default_currency_id.ok_or_else(|| AppError::BadRequest("Please set your default currency in settings first.".to_string()))?;

        let currency = self
            .get_currency_by_id(&currency_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Default currency {} not found", currency_id)))?;

        let account_type_str = account_type_to_db(&request.account_type);

        #[derive(sqlx::FromRow)]
        struct CreateAccountRow {
            id: Uuid,
            name: String,
            color: String,
            icon: String,
            account_type: String,
            balance: i64,
            spend_limit: Option<i32>,
            is_archived: bool,
            next_transfer_amount: Option<i64>,
        }

        let row = sqlx::query_as::<_, CreateAccountRow>(
            r#"
            INSERT INTO account (user_id, name, color, icon, account_type, currency_id, balance, spend_limit, next_transfer_amount)
            VALUES ($1, $2, $3, $4, $5::text::account_type, $6, $7, $8, $9)
            RETURNING
                id,
                name,
                color,
                icon,
                account_type::text as account_type,
                balance,
                spend_limit,
                is_archived,
                next_transfer_amount
            "#,
        )
        .bind(user_id)
        .bind(&request.name)
        .bind(&request.color)
        .bind(&request.icon)
        .bind(&account_type_str)
        .bind(currency_id)
        .bind(request.balance)
        .bind(request.spend_limit)
        .bind(request.next_transfer_amount)
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
            name: row.name,
            color: row.color,
            icon: row.icon,
            account_type: account_type_from_db(&row.account_type),
            currency,
            balance: row.balance,
            spend_limit: row.spend_limit,
            is_archived: row.is_archived,
            next_transfer_amount: row.next_transfer_amount,
        })
    }

    pub async fn get_account_by_id(&self, id: &Uuid, user_id: &Uuid) -> Result<Option<Account>, AppError> {
        let row = sqlx::query_as::<_, AccountRow>(
            r#"
            SELECT
                a.id,
                a.name,
                a.color,
                a.icon,
                a.account_type::text as account_type,
                a.balance,
                a.spend_limit,
                a.is_archived,
                a.next_transfer_amount,
                c.id as currency_id,
                c.name as currency_name,
                c.symbol as currency_symbol,
                c.currency as currency_code,
                c.decimal_places as currency_decimal_places,
                c.symbol_position as currency_symbol_position
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
                    a.name,
                    a.color,
                    a.icon,
                    a.account_type::text as account_type,
                    a.balance,
                    a.spend_limit,
                    a.is_archived,
                    a.next_transfer_amount,
                    c.id as currency_id,
                    c.name as currency_name,
                    c.symbol as currency_symbol,
                    c.currency as currency_code,
                    c.decimal_places as currency_decimal_places,
                    c.symbol_position as currency_symbol_position,
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
                ) AND a.user_id = $3 AND a.is_archived = FALSE
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
                    a.is_archived,
                    a.next_transfer_amount,
                    c.id,
                    c.name,
                    c.symbol,
                    c.currency,
                    c.decimal_places,
                    c.symbol_position
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
                    a.name,
                    a.color,
                    a.icon,
                    a.account_type::text as account_type,
                    a.balance,
                    a.spend_limit,
                    a.is_archived,
                    a.next_transfer_amount,
                    c.id as currency_id,
                    c.name as currency_name,
                    c.symbol as currency_symbol,
                    c.currency as currency_code,
                    c.decimal_places as currency_decimal_places,
                    c.symbol_position as currency_symbol_position,
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
                WHERE a.user_id = $2 AND a.is_archived = FALSE
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
                    a.is_archived,
                    a.next_transfer_amount,
                    c.id,
                    c.name,
                    c.symbol,
                    c.currency,
                    c.decimal_places,
                    c.symbol_position
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

    pub async fn list_accounts_management(&self, user_id: &Uuid) -> Result<Vec<AccountManagementResponse>, AppError> {
        let rows = sqlx::query_as::<_, AccountManagementRow>(
            r#"
            SELECT
                a.id,
                a.name,
                a.color,
                a.icon,
                a.account_type::text as account_type,
                a.balance,
                a.spend_limit,
                a.is_archived,
                a.next_transfer_amount,
                c.id as currency_id,
                c.name as currency_name,
                c.symbol as currency_symbol,
                c.currency as currency_code,
                c.decimal_places as currency_decimal_places,
                c.symbol_position as currency_symbol_position,
                COALESCE(tx.tx_count, 0)::bigint AS transaction_count,
                (COALESCE(tx.tx_count, 0) = 0) AS can_delete,
                (
                    SELECT COALESCE(
                        (
                            SELECT bp.end_date >= CURRENT_DATE
                            FROM budget_period bp
                            WHERE bp.user_id = $1
                            ORDER BY bp.start_date ASC
                            LIMIT 1
                        ),
                        FALSE
                    )
                ) AS can_adjust_balance
            FROM account a
            JOIN currency c ON c.id = a.currency_id
            LEFT JOIN (
                SELECT
                    COALESCE(from_account_id, to_account_id) AS acct_id,
                    COUNT(*) AS tx_count
                FROM transaction
                WHERE user_id = $1
                GROUP BY COALESCE(from_account_id, to_account_id)
            ) tx ON tx.acct_id = a.id
            WHERE a.user_id = $1
            ORDER BY a.is_archived ASC, a.account_type::text ASC, a.name ASC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| AccountManagementResponse {
                id: row.id,
                name: row.name,
                color: row.color,
                icon: row.icon,
                account_type: account_type_from_db(&row.account_type),
                currency: CurrencyResponse {
                    id: row.currency_id,
                    name: row.currency_name,
                    symbol: row.currency_symbol,
                    currency: row.currency_code,
                    decimal_places: row.currency_decimal_places,
                    symbol_position: row.currency_symbol_position,
                },
                balance: row.balance,
                spend_limit: row.spend_limit,
                is_archived: row.is_archived,
                next_transfer_amount: row.next_transfer_amount,
                transaction_count: row.transaction_count,
                can_delete: row.can_delete,
                can_adjust_balance: row.can_adjust_balance,
            })
            .collect())
    }

    pub async fn archive_account(&self, id: &Uuid, user_id: &Uuid) -> Result<(), AppError> {
        let affected = sqlx::query("UPDATE account SET is_archived = TRUE WHERE id = $1 AND user_id = $2 AND is_archived = FALSE")
            .bind(id)
            .bind(user_id)
            .execute(&self.pool)
            .await?
            .rows_affected();

        if affected == 0 {
            return Err(AppError::NotFound("Account not found or already archived".to_string()));
        }

        Ok(())
    }

    pub async fn restore_account(&self, id: &Uuid, user_id: &Uuid) -> Result<(), AppError> {
        let affected = sqlx::query("UPDATE account SET is_archived = FALSE WHERE id = $1 AND user_id = $2 AND is_archived = TRUE")
            .bind(id)
            .bind(user_id)
            .execute(&self.pool)
            .await?
            .rows_affected();

        if affected == 0 {
            return Err(AppError::NotFound("Account not found or not archived".to_string()));
        }

        Ok(())
    }

    pub async fn delete_account(&self, id: &Uuid, user_id: &Uuid) -> Result<(), AppError> {
        // Guard: block deletion if transactions exist for this account
        let has_transactions: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS (
                SELECT 1
                FROM transaction
                WHERE user_id = $1 AND (from_account_id = $2 OR to_account_id = $2)
            )
            "#,
        )
        .bind(user_id)
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        if has_transactions {
            return Err(AppError::BadRequest(
                "Cannot delete account with existing transactions. Archive it instead.".to_string(),
            ));
        }

        sqlx::query("DELETE FROM account WHERE id = $1 AND user_id = $2")
            .bind(id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn adjust_starting_balance(&self, id: &Uuid, new_balance: i64, user_id: &Uuid) -> Result<Account, AppError> {
        // Check that the earliest budget period is still open (end_date >= today)
        let earliest_period_open: Option<bool> = sqlx::query_scalar(
            r#"
            SELECT end_date >= CURRENT_DATE
            FROM budget_period
            WHERE user_id = $1
            ORDER BY start_date ASC
            LIMIT 1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        match earliest_period_open {
            Some(true) => {}
            Some(false) => {
                return Err(AppError::BadRequest(
                    "Cannot adjust starting balance: earliest budget period is already closed.".to_string(),
                ));
            }
            None => {
                return Err(AppError::BadRequest("Cannot adjust starting balance: no budget periods found.".to_string()));
            }
        }

        let row = sqlx::query_as::<_, AccountRow>(
            r#"
            UPDATE account
            SET balance = $1
            WHERE id = $2 AND user_id = $3
            RETURNING
                id,
                name,
                color,
                icon,
                account_type::text as account_type,
                balance,
                spend_limit,
                is_archived,
                next_transfer_amount,
                (SELECT cu.id FROM currency cu WHERE cu.id = currency_id) as currency_id,
                (SELECT cu.name FROM currency cu WHERE cu.id = currency_id) as currency_name,
                (SELECT cu.symbol FROM currency cu WHERE cu.id = currency_id) as currency_symbol,
                (SELECT cu.currency FROM currency cu WHERE cu.id = currency_id) as currency_code,
                (SELECT cu.decimal_places FROM currency cu WHERE cu.id = currency_id) as currency_decimal_places,
                (SELECT cu.symbol_position FROM currency cu WHERE cu.id = currency_id) as currency_symbol_position
            "#,
        )
        .bind(new_balance)
        .bind(id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Account not found".to_string()))?;

        Ok(Account::from(row))
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
            WHERE user_id = $1 AND is_archived = FALSE
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
            WHERE user_id = $1 AND is_archived = FALSE
            "#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        let total_net_worth = row.total_assets - row.total_liabilities;
        Ok((total_net_worth, row.total_assets, row.total_liabilities))
    }

    pub async fn update_account(&self, id: &Uuid, request: &AccountUpdateRequest, user_id: &Uuid) -> Result<Account, AppError> {
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

        // We re-fetch the existing account to get the current currency
        let existing_account = self
            .get_account_by_id(id, user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Account not found".to_string()))?;
        let currency = existing_account.currency;

        let account_type_str = account_type_to_db(&request.account_type);

        #[derive(sqlx::FromRow)]
        struct UpdateAccountRow {
            id: Uuid,
            name: String,
            color: String,
            icon: String,
            account_type: String,
            balance: i64,
            spend_limit: Option<i32>,
            is_archived: bool,
            next_transfer_amount: Option<i64>,
        }

        let row = sqlx::query_as::<_, UpdateAccountRow>(
            r#"
            UPDATE account
            SET name = $1, color = $2, icon = $3, account_type = $4::text::account_type, spend_limit = $5, next_transfer_amount = $6
            WHERE id = $7 AND user_id = $8
            RETURNING
                id,
                name,
                color,
                icon,
                account_type::text as account_type,
                balance,
                spend_limit,
                is_archived,
                next_transfer_amount
            "#,
        )
        .bind(&request.name)
        .bind(&request.color)
        .bind(&request.icon)
        .bind(&account_type_str)
        .bind(request.spend_limit)
        .bind(request.next_transfer_amount)
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
            name: row.name,
            color: row.color,
            icon: row.icon,
            account_type: account_type_from_db(&row.account_type),
            currency,
            balance: row.balance,
            spend_limit: row.spend_limit,
            is_archived: row.is_archived,
            next_transfer_amount: row.next_transfer_amount,
        })
    }

    pub async fn get_account_detail(&self, account_id: &Uuid, period_id: &Uuid, user_id: &Uuid) -> Result<AccountDetailResponse, AppError> {
        #[derive(sqlx::FromRow)]
        struct DetailRow {
            balance: i64,
            inflows: i64,
            outflows: i64,
            period_start: NaiveDate,
            period_end: NaiveDate,
        }

        let row = sqlx::query_as::<_, DetailRow>(
            r#"
WITH period AS (
    SELECT start_date, end_date
    FROM budget_period
    WHERE id = $2 AND user_id = $3
),
period_txs AS (
    SELECT
        t.amount,
        c.category_type,
        t.from_account_id,
        t.to_account_id
    FROM transaction t
    JOIN category c ON c.id = t.category_id
    CROSS JOIN period
    WHERE t.user_id = $3
      AND (t.from_account_id = $1 OR t.to_account_id = $1)
      AND t.occurred_at >= period.start_date
      AND t.occurred_at <= period.end_date
),
flow AS (
    SELECT
        COALESCE(SUM(CASE
            WHEN category_type = 'Incoming' THEN amount
            WHEN category_type = 'Transfer' AND to_account_id = $1 THEN amount
            ELSE 0
        END), 0) AS inflows,
        COALESCE(SUM(CASE
            WHEN category_type = 'Outgoing' THEN amount
            WHEN category_type = 'Transfer' AND from_account_id = $1 THEN amount
            ELSE 0
        END), 0) AS outflows
    FROM period_txs
)
SELECT
    a.balance                         AS balance,
    flow.inflows::bigint              AS inflows,
    flow.outflows::bigint             AS outflows,
    period.start_date                 AS period_start,
    period.end_date                   AS period_end
FROM account a
CROSS JOIN flow
CROSS JOIN period
WHERE a.id = $1 AND a.user_id = $3
            "#,
        )
        .bind(account_id)
        .bind(period_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Account not found".to_string()))?;

        let net = row.inflows - row.outflows;
        let balance_change = net;
        Ok(AccountDetailResponse {
            balance: row.balance,
            balance_change,
            inflows: row.inflows,
            outflows: row.outflows,
            net,
            period_start: row.period_start,
            period_end: row.period_end,
        })
    }

    pub async fn get_account_balance_history(
        &self,
        account_id: &Uuid,
        start_date: NaiveDate,
        end_date: NaiveDate,
        user_id: &Uuid,
    ) -> Result<Vec<AccountBalanceHistoryPoint>, AppError> {
        #[derive(sqlx::FromRow)]
        struct HistoryRow {
            date: String,
            balance: i64,
        }

        let rows = sqlx::query_as::<_, HistoryRow>(
            r#"
WITH days AS (
    SELECT d::date AS day
    FROM generate_series($2::date, $3::date, '1 day') AS d
),
base_balance AS (
    SELECT
        a.balance + COALESCE(SUM(
            CASE
                WHEN c.category_type = 'Incoming'                              THEN  t.amount::bigint
                WHEN c.category_type = 'Outgoing'                              THEN -t.amount::bigint
                WHEN c.category_type = 'Transfer' AND t.from_account_id = a.id THEN -t.amount::bigint
                WHEN c.category_type = 'Transfer' AND t.to_account_id   = a.id THEN  t.amount::bigint
                ELSE 0
            END
        ), 0) AS base_bal
    FROM account a
    LEFT JOIN transaction t ON (t.from_account_id = a.id OR t.to_account_id = a.id)
                            AND t.occurred_at < $2
                            AND t.user_id = $4
    LEFT JOIN category c ON t.category_id = c.id
    WHERE a.id = $1 AND a.user_id = $4
    GROUP BY a.id, a.balance
),
daily_totals AS (
    SELECT
        t.occurred_at::date AS day,
        SUM(
            CASE
                WHEN c.category_type = 'Incoming'                              THEN  t.amount::bigint
                WHEN c.category_type = 'Outgoing'                              THEN -t.amount::bigint
                WHEN c.category_type = 'Transfer' AND t.from_account_id = $1  THEN -t.amount::bigint
                WHEN c.category_type = 'Transfer' AND t.to_account_id   = $1  THEN  t.amount::bigint
                ELSE 0
            END
        ) AS daily_amount
    FROM transaction t
    JOIN category c ON c.id = t.category_id
    WHERE (t.from_account_id = $1 OR t.to_account_id = $1)
      AND t.user_id = $4
      AND t.occurred_at >= $2
      AND t.occurred_at <= $3
    GROUP BY t.occurred_at::date
)
SELECT
    to_char(d.day, 'YYYY-MM-DD') AS date,
    (bb.base_bal + SUM(COALESCE(dt.daily_amount, 0)) OVER (
        ORDER BY d.day
        ROWS UNBOUNDED PRECEDING
    ))::bigint AS balance
FROM days d
CROSS JOIN base_balance bb
LEFT JOIN daily_totals dt ON dt.day = d.day
ORDER BY d.day
            "#,
        )
        .bind(account_id)
        .bind(start_date)
        .bind(end_date)
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| AccountBalanceHistoryPoint {
                date: r.date,
                balance: r.balance,
            })
            .collect())
    }

    pub async fn get_account_transactions(
        &self,
        account_id: &Uuid,
        period_id: &Uuid,
        flow_filter: Option<&str>,
        params: &CursorParams,
        user_id: &Uuid,
    ) -> Result<Vec<AccountTransactionResponse>, AppError> {
        #[derive(sqlx::FromRow)]
        struct TxRow {
            id: Uuid,
            amount: i64,
            description: String,
            occurred_at: NaiveDate,
            category_name: String,
            category_color: String,
            flow: String,
            running_balance: i64,
        }

        let flow_clause = match flow_filter {
            Some("in") => "AND flow = 'in'",
            Some("out") => "AND flow = 'out'",
            _ => "",
        };

        // Build cursor clause using the same subquery pattern as list_transactions
        let (cursor_clause, has_cursor) = if params.cursor.is_some() {
            ("AND (occurred_at, id) < (SELECT occurred_at, id FROM transaction WHERE id = $5)", true)
        } else {
            ("", false)
        };

        let sql = format!(
            r#"
WITH period AS (
    SELECT start_date, end_date FROM budget_period WHERE id = $2 AND user_id = $4
),
base_balance AS (
    SELECT
        a.balance + COALESCE(SUM(
            CASE
                WHEN c.category_type = 'Incoming'                              THEN  t.amount::bigint
                WHEN c.category_type = 'Outgoing'                              THEN -t.amount::bigint
                WHEN c.category_type = 'Transfer' AND t.from_account_id = a.id THEN -t.amount::bigint
                WHEN c.category_type = 'Transfer' AND t.to_account_id   = a.id THEN  t.amount::bigint
                ELSE 0
            END
        ), 0) AS base_bal
    FROM account a
    LEFT JOIN transaction t  ON (t.from_account_id = a.id OR t.to_account_id = a.id)
                             AND t.occurred_at < (SELECT start_date FROM period)
                             AND t.user_id = $4
    LEFT JOIN category c ON t.category_id = c.id
    WHERE a.id = $1 AND a.user_id = $4
    GROUP BY a.id, a.balance
),
period_txs AS (
    SELECT
        t.id,
        t.amount::bigint AS amount,
        t.description,
        t.occurred_at,
        cat.name  AS category_name,
        cat.color AS category_color,
        CASE
            WHEN cat.category_type = 'Incoming'                              THEN 'in'
            WHEN cat.category_type = 'Transfer' AND t.to_account_id = $1    THEN 'in'
            ELSE 'out'
        END AS flow
    FROM transaction t
    JOIN category cat ON cat.id = t.category_id
    CROSS JOIN period
    WHERE (t.from_account_id = $1 OR t.to_account_id = $1)
      AND t.user_id = $4
      AND t.occurred_at >= period.start_date
      AND t.occurred_at <= period.end_date
),
with_running AS (
    SELECT
        pt.*,
        (bb.base_bal + SUM(
            CASE WHEN flow = 'in' THEN amount ELSE -amount END
        ) OVER (ORDER BY occurred_at ASC, id ASC ROWS UNBOUNDED PRECEDING))::bigint AS running_balance
    FROM period_txs pt
    CROSS JOIN base_balance bb
)
SELECT * FROM with_running
WHERE 1=1 {flow_clause} {cursor_clause}
ORDER BY occurred_at DESC, id DESC
LIMIT $3
            "#,
            flow_clause = flow_clause,
            cursor_clause = cursor_clause,
        );

        let rows = if has_cursor {
            let cursor_id = params.cursor.unwrap();
            sqlx::query_as::<_, TxRow>(&sql)
                .bind(account_id)
                .bind(period_id)
                .bind(params.fetch_limit())
                .bind(user_id)
                .bind(cursor_id)
                .fetch_all(&self.pool)
                .await?
        } else {
            sqlx::query_as::<_, TxRow>(&sql)
                .bind(account_id)
                .bind(period_id)
                .bind(params.fetch_limit())
                .bind(user_id)
                .fetch_all(&self.pool)
                .await?
        };

        Ok(rows
            .into_iter()
            .map(|r| AccountTransactionResponse {
                id: r.id,
                amount: r.amount,
                description: r.description,
                occurred_at: r.occurred_at,
                category_name: r.category_name,
                category_color: r.category_color,
                flow: r.flow,
                running_balance: r.running_balance,
            })
            .collect())
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

pub fn account_type_to_db(account_type: &AccountType) -> String {
    match account_type {
        AccountType::Checking => "Checking".to_string(),
        AccountType::Savings => "Savings".to_string(),
        AccountType::CreditCard => "CreditCard".to_string(),
        AccountType::Wallet => "Wallet".to_string(),
        AccountType::Allowance => "Allowance".to_string(),
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
            balance: 0,
            spend_limit: None,
            next_transfer_amount: None,
        };
        assert_eq!(account_type_to_db(&request.account_type), "Checking");

        let request_savings = AccountRequest {
            name: "Test".to_string(),
            color: "#000000".to_string(),
            icon: "icon".to_string(),
            account_type: AccountType::Savings,
            balance: 0,
            spend_limit: None,
            next_transfer_amount: None,
        };
        assert_eq!(account_type_to_db(&request_savings.account_type), "Savings");

        let request_credit = AccountRequest {
            name: "Test".to_string(),
            color: "#000000".to_string(),
            icon: "icon".to_string(),
            account_type: AccountType::CreditCard,
            balance: 0,
            spend_limit: None,
            next_transfer_amount: None,
        };
        assert_eq!(account_type_to_db(&request_credit.account_type), "CreditCard");
    }
}
