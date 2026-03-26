use crate::database::postgres_repository::{PostgresRepository, is_unique_violation};
use crate::error::app_error::AppError;
use crate::models::account::{
    Account, AccountBalanceHistoryPoint, AccountBalancePerDay, AccountContextResponse, AccountDetailResponse, AccountManagementResponse, AccountRequest,
    AccountStability, AccountTransactionResponse, AccountType, AccountUpdateRequest, AccountWithMetrics, CategoryImpactItem,
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
    top_up_amount: Option<i64>,
    top_up_cycle: Option<String>,
    top_up_day: Option<i32>,
    statement_close_day: Option<i32>,
    payment_due_day: Option<i32>,
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
            top_up_amount: row.top_up_amount,
            top_up_cycle: row.top_up_cycle,
            top_up_day: row.top_up_day,
            statement_close_day: row.statement_close_day,
            payment_due_day: row.payment_due_day,
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
    top_up_amount: Option<i64>,
    top_up_cycle: Option<String>,
    top_up_day: Option<i32>,
    statement_close_day: Option<i32>,
    payment_due_day: Option<i32>,
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
                top_up_amount: row.top_up_amount,
                top_up_cycle: row.top_up_cycle,
                top_up_day: row.top_up_day,
                statement_close_day: row.statement_close_day,
                payment_due_day: row.payment_due_day,
            },
            current_balance: row.current_balance,
            balance_change_this_period: row.balance_change_this_period,
            transaction_count: row.transaction_count,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
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
    top_up_amount: Option<i64>,
    top_up_cycle: Option<String>,
    top_up_day: Option<i32>,
    statement_close_day: Option<i32>,
    payment_due_day: Option<i32>,
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
            top_up_amount: Option<i64>,
            top_up_cycle: Option<String>,
            top_up_day: Option<i32>,
            statement_close_day: Option<i32>,
            payment_due_day: Option<i32>,
        }

        let row = sqlx::query_as::<_, CreateAccountRow>(
            r#"
            INSERT INTO account (user_id, name, color, icon, account_type, currency_id, balance, spend_limit, next_transfer_amount,
                                 top_up_amount, top_up_cycle, top_up_day, statement_close_day, payment_due_day)
            VALUES ($1, $2, $3, $4, $5::text::account_type, $6, $7, $8, $9, $10, $11, $12, $13, $14)
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
                top_up_amount,
                top_up_cycle,
                top_up_day::int as top_up_day,
                statement_close_day::int as statement_close_day,
                payment_due_day::int as payment_due_day
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
        .bind(request.top_up_amount)
        .bind(request.top_up_cycle.as_deref())
        .bind(request.top_up_day)
        .bind(request.statement_close_day)
        .bind(request.payment_due_day)
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
            top_up_amount: row.top_up_amount,
            top_up_cycle: row.top_up_cycle,
            top_up_day: row.top_up_day,
            statement_close_day: row.statement_close_day,
            payment_due_day: row.payment_due_day,
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
                c.symbol_position as currency_symbol_position,
                a.top_up_amount,
                a.top_up_cycle,
                a.top_up_day::int as top_up_day,
                a.statement_close_day::int as statement_close_day,
                a.payment_due_day::int as payment_due_day
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

    /// Get a single account with its computed current_balance from all transactions.
    pub async fn get_account_with_metrics(&self, id: &Uuid, user_id: &Uuid) -> Result<Option<AccountWithMetrics>, AppError> {
        let row = sqlx::query_as::<_, AccountMetricsRow>(
            r#"
            SELECT
                a.id, a.name, a.color, a.icon,
                a.account_type::text as account_type,
                a.balance, a.spend_limit, a.is_archived, a.next_transfer_amount,
                c.id as currency_id, c.name as currency_name, c.symbol as currency_symbol,
                c.currency as currency_code, c.decimal_places as currency_decimal_places,
                c.symbol_position as currency_symbol_position,
                a.top_up_amount, a.top_up_cycle,
                a.top_up_day::int as top_up_day,
                a.statement_close_day::int as statement_close_day,
                a.payment_due_day::int as payment_due_day,
                (a.balance + COALESCE(SUM(
                    CASE
                        WHEN cat.category_type = 'Incoming'                              THEN  t.amount::bigint
                        WHEN cat.category_type = 'Outgoing'                              THEN -t.amount::bigint
                        WHEN cat.category_type = 'Transfer' AND t.from_account_id = a.id THEN -t.amount::bigint
                        WHEN cat.category_type = 'Transfer' AND t.to_account_id   = a.id THEN  t.amount::bigint
                        ELSE 0
                    END
                ), 0))::bigint AS current_balance,
                0::bigint AS balance_change_this_period,
                COUNT(t.id)::bigint AS transaction_count
            FROM account a
            JOIN currency c ON c.id = a.currency_id
            LEFT JOIN transaction t ON (t.from_account_id = a.id OR t.to_account_id = a.id) AND t.user_id = $2
            LEFT JOIN category cat ON t.category_id = cat.id
            WHERE a.id = $1 AND a.user_id = $2
            GROUP BY a.id, a.name, a.color, a.icon, a.account_type, a.balance,
                     a.spend_limit, a.is_archived, a.next_transfer_amount,
                     a.top_up_amount, a.top_up_cycle, a.top_up_day, a.statement_close_day, a.payment_due_day,
                     c.id, c.name, c.symbol, c.currency, c.decimal_places, c.symbol_position
            "#,
        )
        .bind(id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(AccountWithMetrics::from))
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
                    a.top_up_amount,
                    a.top_up_cycle,
                    a.top_up_day::int as top_up_day,
                    a.statement_close_day::int as statement_close_day,
                    a.payment_due_day::int as payment_due_day,
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
                    COUNT(t.id) FILTER (WHERE p.start_date IS NOT NULL AND t.occurred_at >= p.start_date AND t.occurred_at <= p.end_date)::bigint AS transaction_count
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
                    a.top_up_amount,
                    a.top_up_cycle,
                    a.top_up_day,
                    a.statement_close_day,
                    a.payment_due_day,
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
                    a.top_up_amount,
                    a.top_up_cycle,
                    a.top_up_day::int as top_up_day,
                    a.statement_close_day::int as statement_close_day,
                    a.payment_due_day::int as payment_due_day,
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
                    COUNT(t.id) FILTER (WHERE p.start_date IS NOT NULL AND t.occurred_at >= p.start_date AND t.occurred_at <= p.end_date)::bigint AS transaction_count
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
                    a.top_up_amount,
                    a.top_up_cycle,
                    a.top_up_day,
                    a.statement_close_day,
                    a.payment_due_day,
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
                a.top_up_amount,
                a.top_up_cycle,
                a.top_up_day::int as top_up_day,
                a.statement_close_day::int as statement_close_day,
                a.payment_due_day::int as payment_due_day,
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
        let is_archived: Option<bool> = sqlx::query_scalar("SELECT is_archived FROM account WHERE id = $1 AND user_id = $2")
            .bind(id)
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await?;

        match is_archived {
            None => return Err(AppError::NotFound("Account not found".to_string())),
            Some(true) => return Err(AppError::Conflict("Account is already archived".to_string())),
            Some(false) => {}
        }

        sqlx::query("UPDATE account SET is_archived = TRUE WHERE id = $1 AND user_id = $2")
            .bind(id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn restore_account(&self, id: &Uuid, user_id: &Uuid) -> Result<(), AppError> {
        let is_archived: Option<bool> = sqlx::query_scalar("SELECT is_archived FROM account WHERE id = $1 AND user_id = $2")
            .bind(id)
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await?;

        match is_archived {
            None => return Err(AppError::NotFound("Account not found".to_string())),
            Some(false) => return Err(AppError::Conflict("Account is already active".to_string())),
            Some(true) => {}
        }

        sqlx::query("UPDATE account SET is_archived = FALSE WHERE id = $1 AND user_id = $2")
            .bind(id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

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
                top_up_amount,
                top_up_cycle,
                top_up_day::int as top_up_day,
                statement_close_day::int as statement_close_day,
                payment_due_day::int as payment_due_day,
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
            top_up_amount: Option<i64>,
            top_up_cycle: Option<String>,
            top_up_day: Option<i32>,
            statement_close_day: Option<i32>,
            payment_due_day: Option<i32>,
        }

        let row = sqlx::query_as::<_, UpdateAccountRow>(
            r#"
            UPDATE account
            SET name = $1, color = $2, icon = $3, account_type = $4::text::account_type, balance = $5, spend_limit = $6, next_transfer_amount = $7,
                top_up_amount = $8, top_up_cycle = $9, top_up_day = $10, statement_close_day = $11, payment_due_day = $12
            WHERE id = $13 AND user_id = $14
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
                top_up_amount,
                top_up_cycle,
                top_up_day::int as top_up_day,
                statement_close_day::int as statement_close_day,
                payment_due_day::int as payment_due_day
            "#,
        )
        .bind(&request.name)
        .bind(&request.color)
        .bind(&request.icon)
        .bind(&account_type_str)
        .bind(request.balance)
        .bind(request.spend_limit)
        .bind(request.next_transfer_amount)
        .bind(request.top_up_amount)
        .bind(request.top_up_cycle.as_deref())
        .bind(request.top_up_day)
        .bind(request.statement_close_day)
        .bind(request.payment_due_day)
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
            top_up_amount: row.top_up_amount,
            top_up_cycle: row.top_up_cycle,
            top_up_day: row.top_up_day,
            statement_close_day: row.statement_close_day,
            payment_due_day: row.payment_due_day,
        })
    }

    pub async fn get_account_detail(&self, account_id: &Uuid, period_id: &Uuid, user_id: &Uuid) -> Result<AccountDetailResponse, AppError> {
        #[derive(sqlx::FromRow)]
        struct DetailRow {
            balance: i64,
            inflows: i64,
            outflows: i64,
            transaction_count: i64,
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
        END), 0) AS outflows,
        COUNT(*)::bigint AS transaction_count
    FROM period_txs
)
SELECT
    a.balance                         AS balance,
    flow.inflows::bigint              AS inflows,
    flow.outflows::bigint             AS outflows,
    flow.transaction_count::bigint    AS transaction_count,
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
            transaction_count: row.transaction_count,
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
            transaction_count: i64,
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
        ) AS daily_amount,
        COUNT(*)::bigint AS tx_count
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
    ))::bigint AS balance,
    COALESCE(dt.tx_count, 0)::bigint AS transaction_count
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
                transaction_count: r.transaction_count,
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

    pub async fn get_account_context(&self, account_id: &Uuid, period_id: &Uuid, user_id: &Uuid) -> Result<AccountContextResponse, AppError> {
        // --- Category impact: outflows in current period, grouped by category ---
        #[derive(sqlx::FromRow)]
        struct CategoryRow {
            category_id: Uuid,
            category_name: String,
            amount: i64,
        }

        let cat_rows = sqlx::query_as::<_, CategoryRow>(
            r#"
WITH period AS (
    SELECT start_date, end_date FROM budget_period WHERE id = $2 AND user_id = $3
)
SELECT
    cat.id AS category_id,
    cat.name AS category_name,
    SUM(t.amount)::bigint AS amount
FROM transaction t
JOIN category cat ON cat.id = t.category_id
CROSS JOIN period
WHERE t.from_account_id = $1
  AND t.user_id = $3
  AND cat.category_type = 'Outgoing'
  AND t.occurred_at >= period.start_date
  AND t.occurred_at <= period.end_date
GROUP BY cat.id, cat.name
ORDER BY amount DESC
            "#,
        )
        .bind(account_id)
        .bind(period_id)
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        let total_outflows: i64 = cat_rows.iter().map(|r| r.amount).sum();
        let category_impact = cat_rows
            .into_iter()
            .map(|r| {
                let pct = if total_outflows > 0 { (r.amount * 100 / total_outflows) as i32 } else { 0 };
                CategoryImpactItem {
                    category_id: r.category_id,
                    category_name: r.category_name,
                    amount: r.amount,
                    percentage: pct,
                }
            })
            .collect();

        // --- Stability: look at up to 6 prior closed periods ---
        #[derive(sqlx::FromRow)]
        struct StabilityRow {
            closing_balance: i64,
            is_positive: bool,
        }

        let stability_rows = sqlx::query_as::<_, StabilityRow>(
            r#"
WITH prior_periods AS (
    SELECT id, start_date, end_date
    FROM budget_period
    WHERE user_id = $2
      AND end_date < CURRENT_DATE
      AND id != $3
    ORDER BY end_date DESC
    LIMIT 6
),
period_flows AS (
    SELECT
        pp.id AS period_id,
        pp.end_date,
        COALESCE(SUM(
            CASE
                WHEN c.category_type = 'Incoming'                              THEN  t.amount::bigint
                WHEN c.category_type = 'Outgoing'                              THEN -t.amount::bigint
                WHEN c.category_type = 'Transfer' AND t.from_account_id = $1  THEN -t.amount::bigint
                WHEN c.category_type = 'Transfer' AND t.to_account_id   = $1  THEN  t.amount::bigint
                ELSE 0
            END
        ), 0) AS net_flow
    FROM prior_periods pp
    LEFT JOIN transaction t ON (t.from_account_id = $1 OR t.to_account_id = $1)
                            AND t.user_id = $2
                            AND t.occurred_at >= pp.start_date
                            AND t.occurred_at <= pp.end_date
    LEFT JOIN category c ON c.id = t.category_id
    GROUP BY pp.id, pp.end_date
),
base AS (
    SELECT a.balance AS current_balance FROM account a WHERE a.id = $1 AND a.user_id = $2
)
SELECT
    (base.current_balance + SUM(pf.net_flow) OVER (ORDER BY pf.end_date DESC ROWS UNBOUNDED PRECEDING))::bigint AS closing_balance,
    ((base.current_balance + SUM(pf.net_flow) OVER (ORDER BY pf.end_date DESC ROWS UNBOUNDED PRECEDING)) > 0) AS is_positive
FROM period_flows pf
CROSS JOIN base
            "#,
        )
        .bind(account_id)
        .bind(user_id)
        .bind(period_id)
        .fetch_all(&self.pool)
        .await?;

        let periods_evaluated = stability_rows.len() as i64;
        let periods_closed_positive = stability_rows.iter().filter(|r| r.is_positive).count() as i64;
        let balances: Vec<i64> = stability_rows.iter().map(|r| r.closing_balance).collect();
        let avg = if periods_evaluated > 0 {
            balances.iter().sum::<i64>() / periods_evaluated
        } else {
            0
        };
        let highest = balances.iter().copied().max().unwrap_or(0);
        let lowest = balances.iter().copied().min().unwrap_or(0);

        // Largest single outflow in current period
        #[derive(sqlx::FromRow)]
        struct OutflowRow {
            category_name: String,
            amount: i64,
        }

        let largest = sqlx::query_as::<_, OutflowRow>(
            r#"
WITH period AS (
    SELECT start_date, end_date FROM budget_period WHERE id = $2 AND user_id = $3
)
SELECT
    cat.name AS category_name,
    t.amount::bigint AS amount
FROM transaction t
JOIN category cat ON cat.id = t.category_id
CROSS JOIN period
WHERE t.from_account_id = $1
  AND t.user_id = $3
  AND cat.category_type = 'Outgoing'
  AND t.occurred_at >= period.start_date
  AND t.occurred_at <= period.end_date
ORDER BY t.amount DESC
LIMIT 1
            "#,
        )
        .bind(account_id)
        .bind(period_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        let (largest_outflow, largest_outflow_cat) = largest.map(|r| (r.amount, r.category_name)).unwrap_or((0, String::new()));

        Ok(AccountContextResponse {
            category_impact,
            stability: AccountStability {
                periods_closed_positive,
                periods_evaluated,
                avg_closing_balance: avg,
                highest_closing_balance: highest,
                lowest_closing_balance: lowest,
                largest_single_outflow: largest_outflow,
                largest_single_outflow_category: largest_outflow_cat,
            },
        })
    }

    // ===== V2-specific methods =====

    /// Simple paginated list of accounts (no period metrics). Returns (accounts, total_count).
    pub async fn list_accounts_v2(&self, cursor: Option<Uuid>, limit: i64, user_id: &Uuid) -> Result<(Vec<Account>, i64), AppError> {
        let total_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM account WHERE user_id = $1 AND is_archived = FALSE")
            .bind(user_id)
            .fetch_one(&self.pool)
            .await?;

        let fetch_limit = limit + 1;
        let rows = if let Some(cursor_id) = cursor {
            sqlx::query_as::<_, AccountRow>(
                r#"
                SELECT
                    a.id, a.name, a.color, a.icon,
                    a.account_type::text as account_type,
                    a.balance, a.spend_limit, a.is_archived, a.next_transfer_amount,
                    a.top_up_amount, a.top_up_cycle,
                    a.top_up_day::int as top_up_day,
                    a.statement_close_day::int as statement_close_day,
                    a.payment_due_day::int as payment_due_day,
                    c.id as currency_id, c.name as currency_name, c.symbol as currency_symbol,
                    c.currency as currency_code, c.decimal_places as currency_decimal_places,
                    c.symbol_position as currency_symbol_position
                FROM account a
                JOIN currency c ON c.id = a.currency_id
                WHERE (a.created_at, a.id) < (SELECT created_at, id FROM account WHERE id = $1)
                  AND a.user_id = $2 AND a.is_archived = FALSE
                ORDER BY a.created_at DESC, a.id DESC
                LIMIT $3
                "#,
            )
            .bind(cursor_id)
            .bind(user_id)
            .bind(fetch_limit)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, AccountRow>(
                r#"
                SELECT
                    a.id, a.name, a.color, a.icon,
                    a.account_type::text as account_type,
                    a.balance, a.spend_limit, a.is_archived, a.next_transfer_amount,
                    a.top_up_amount, a.top_up_cycle,
                    a.top_up_day::int as top_up_day,
                    a.statement_close_day::int as statement_close_day,
                    a.payment_due_day::int as payment_due_day,
                    c.id as currency_id, c.name as currency_name, c.symbol as currency_symbol,
                    c.currency as currency_code, c.decimal_places as currency_decimal_places,
                    c.symbol_position as currency_symbol_position
                FROM account a
                JOIN currency c ON c.id = a.currency_id
                WHERE a.user_id = $1 AND a.is_archived = FALSE
                ORDER BY a.created_at DESC, a.id DESC
                LIMIT $2
                "#,
            )
            .bind(user_id)
            .bind(fetch_limit)
            .fetch_all(&self.pool)
            .await?
        };

        Ok((rows.into_iter().map(Account::from).collect(), total_count))
    }

    /// Get account options for V2 (id, name, color instead of icon).
    pub async fn get_account_options_v2(&self, user_id: &Uuid) -> Result<Vec<(Uuid, String, String)>, AppError> {
        #[derive(sqlx::FromRow)]
        struct Row {
            id: Uuid,
            name: String,
            color: String,
        }

        let rows = sqlx::query_as::<_, Row>(
            r#"
            SELECT id, name, color
            FROM account
            WHERE user_id = $1 AND is_archived = FALSE
            ORDER BY name ASC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| (r.id, r.name, r.color)).collect())
    }

    /// V2 adjust balance: updates the account balance without requiring budget periods.
    pub async fn adjust_balance_v2(&self, id: &Uuid, new_balance: i64, user_id: &Uuid) -> Result<Account, AppError> {
        let row = sqlx::query_as::<_, AccountRow>(
            r#"
            UPDATE account a
            SET balance = $1
            FROM currency c
            WHERE c.id = a.currency_id
              AND a.id = $2 AND a.user_id = $3
            RETURNING
                a.id, a.name, a.color, a.icon,
                a.account_type::text as account_type,
                a.balance, a.spend_limit, a.is_archived, a.next_transfer_amount,
                a.top_up_amount, a.top_up_cycle,
                a.top_up_day::int as top_up_day,
                a.statement_close_day::int as statement_close_day,
                a.payment_due_day::int as payment_due_day,
                c.id as currency_id, c.name as currency_name, c.symbol as currency_symbol,
                c.currency as currency_code, c.decimal_places as currency_decimal_places,
                c.symbol_position as currency_symbol_position
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

    /// Get the current (active) budget period, or the latest one.
    pub async fn get_current_period_id(&self, user_id: &Uuid) -> Result<Option<Uuid>, AppError> {
        let id: Option<Uuid> = sqlx::query_scalar(
            r#"
            SELECT id FROM budget_period
            WHERE user_id = $1
            ORDER BY
                CASE WHEN start_date <= CURRENT_DATE AND end_date >= CURRENT_DATE THEN 0 ELSE 1 END,
                end_date DESC
            LIMIT 1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(id)
    }

    /// Simple paginated list of accounts with summary metrics for V2.
    pub async fn list_accounts_summary_v2(
        &self,
        cursor: Option<Uuid>,
        limit: i64,
        period_id: Option<&Uuid>,
        user_id: &Uuid,
    ) -> Result<(Vec<AccountWithMetrics>, i64), AppError> {
        let total_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM account WHERE user_id = $1 AND is_archived = FALSE")
            .bind(user_id)
            .fetch_one(&self.pool)
            .await?;

        // If we have a period, use the full metrics query (fetch limit+1 for has_more sentinel)
        if let Some(pid) = period_id {
            let params = CursorParams {
                cursor,
                limit: Some(limit + 1),
            };
            let rows = self.list_accounts(&params, pid, user_id).await?;
            return Ok((rows, total_count));
        }

        // No period: compute current_balance and transaction_count from all transactions
        let fetch_limit = limit + 1;
        let rows = if let Some(cursor_id) = cursor {
            sqlx::query_as::<_, AccountMetricsRow>(
                r#"
                SELECT
                    a.id, a.name, a.color, a.icon,
                    a.account_type::text as account_type,
                    a.balance, a.spend_limit, a.is_archived, a.next_transfer_amount,
                    a.top_up_amount, a.top_up_cycle,
                    a.top_up_day::int as top_up_day,
                    a.statement_close_day::int as statement_close_day,
                    a.payment_due_day::int as payment_due_day,
                    c.id as currency_id, c.name as currency_name, c.symbol as currency_symbol,
                    c.currency as currency_code, c.decimal_places as currency_decimal_places,
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
                    0::bigint AS balance_change_this_period,
                    COUNT(t.id)::bigint AS transaction_count
                FROM account a
                JOIN currency c ON c.id = a.currency_id
                LEFT JOIN transaction t ON (t.from_account_id = a.id OR t.to_account_id = a.id) AND t.user_id = $2
                LEFT JOIN category cat ON t.category_id = cat.id
                WHERE (a.created_at, a.id) < (SELECT created_at, id FROM account WHERE id = $1)
                  AND a.user_id = $2 AND a.is_archived = FALSE
                GROUP BY a.id, a.name, a.color, a.icon, a.account_type, a.balance,
                         a.spend_limit, a.is_archived, a.next_transfer_amount, a.created_at,
                         a.top_up_amount, a.top_up_cycle, a.top_up_day, a.statement_close_day, a.payment_due_day,
                         c.id, c.name, c.symbol, c.currency, c.decimal_places, c.symbol_position
                ORDER BY a.created_at DESC, a.id DESC
                LIMIT $3
                "#,
            )
            .bind(cursor_id)
            .bind(user_id)
            .bind(fetch_limit)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, AccountMetricsRow>(
                r#"
                SELECT
                    a.id, a.name, a.color, a.icon,
                    a.account_type::text as account_type,
                    a.balance, a.spend_limit, a.is_archived, a.next_transfer_amount,
                    a.top_up_amount, a.top_up_cycle,
                    a.top_up_day::int as top_up_day,
                    a.statement_close_day::int as statement_close_day,
                    a.payment_due_day::int as payment_due_day,
                    c.id as currency_id, c.name as currency_name, c.symbol as currency_symbol,
                    c.currency as currency_code, c.decimal_places as currency_decimal_places,
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
                    0::bigint AS balance_change_this_period,
                    COUNT(t.id)::bigint AS transaction_count
                FROM account a
                JOIN currency c ON c.id = a.currency_id
                LEFT JOIN transaction t ON (t.from_account_id = a.id OR t.to_account_id = a.id) AND t.user_id = $1
                LEFT JOIN category cat ON t.category_id = cat.id
                WHERE a.user_id = $1 AND a.is_archived = FALSE
                GROUP BY a.id, a.name, a.color, a.icon, a.account_type, a.balance,
                         a.spend_limit, a.is_archived, a.next_transfer_amount, a.created_at,
                         a.top_up_amount, a.top_up_cycle, a.top_up_day, a.statement_close_day, a.payment_due_day,
                         c.id, c.name, c.symbol, c.currency, c.decimal_places, c.symbol_position
                ORDER BY a.created_at DESC, a.id DESC
                LIMIT $2
                "#,
            )
            .bind(user_id)
            .bind(fetch_limit)
            .fetch_all(&self.pool)
            .await?
        };

        Ok((rows.into_iter().map(AccountWithMetrics::from).collect(), total_count))
    }

    /// Compute the total outgoing spend for an Allowance account since the start of its current
    /// top-up cycle.  Returns 0 if the account has no top-up configuration.
    pub async fn get_allowance_spent_this_cycle(&self, account_id: &Uuid, user_id: &Uuid) -> Result<i64, AppError> {
        #[derive(sqlx::FromRow)]
        struct SpentRow {
            spent: i64,
        }

        let row = sqlx::query_as::<_, SpentRow>(
            r#"
WITH account_cfg AS (
    SELECT top_up_cycle, top_up_day
    FROM account
    WHERE id = $1 AND user_id = $2 AND account_type = 'Allowance'
),
cycle_start AS (
    SELECT
        CASE cfg.top_up_cycle
            -- weekly: most recent occurrence of top_up_day (0=Sun … 6=Sat)
            WHEN 'weekly' THEN
                CURRENT_DATE - ((EXTRACT(DOW FROM CURRENT_DATE)::int - cfg.top_up_day + 7) % 7) * INTERVAL '1 day'
            -- bi-weekly: same day-of-week logic but jump back by 14 days if needed
            WHEN 'bi-weekly' THEN
                CURRENT_DATE - ((EXTRACT(DOW FROM CURRENT_DATE)::int - cfg.top_up_day + 7) % 7) * INTERVAL '1 day'
            -- monthly: day cfg.top_up_day of the current month (or last month if not yet reached)
            WHEN 'monthly' THEN
                CASE
                    WHEN EXTRACT(DAY FROM CURRENT_DATE)::int >= cfg.top_up_day
                    THEN DATE_TRUNC('month', CURRENT_DATE) + (cfg.top_up_day - 1) * INTERVAL '1 day'
                    ELSE DATE_TRUNC('month', CURRENT_DATE - INTERVAL '1 month') + (cfg.top_up_day - 1) * INTERVAL '1 day'
                END
            ELSE CURRENT_DATE
        END::date AS start_date
    FROM account_cfg cfg
)
SELECT
    COALESCE(SUM(
        CASE
            WHEN cat.category_type = 'Outgoing' THEN t.amount::bigint
            WHEN cat.category_type = 'Transfer' AND t.from_account_id = $1 THEN t.amount::bigint
            ELSE 0
        END
    ), 0)::bigint AS spent
FROM cycle_start cs
LEFT JOIN transaction t
    ON (t.from_account_id = $1 OR t.to_account_id = $1)
    AND t.user_id = $2
    AND t.occurred_at >= cs.start_date
LEFT JOIN category cat ON cat.id = t.category_id
            "#,
        )
        .bind(account_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map_or(0, |r| r.spent))
    }

    /// Compute the average daily balance for a Checking account over a given period.
    /// Returns 0 if no period is provided or no history is available.
    pub async fn get_avg_daily_balance(&self, account_id: &Uuid, start_date: NaiveDate, end_date: NaiveDate, user_id: &Uuid) -> Result<i64, AppError> {
        #[derive(sqlx::FromRow)]
        struct AvgRow {
            avg_balance: i64,
        }

        let today = chrono::Local::now().date_naive();
        let effective_end = if end_date > today { today } else { end_date };

        if effective_end < start_date {
            return Ok(0);
        }

        let row = sqlx::query_as::<_, AvgRow>(
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
),
daily_balances AS (
    SELECT
        (bb.base_bal + SUM(COALESCE(dt.daily_amount, 0)) OVER (
            ORDER BY d.day
            ROWS UNBOUNDED PRECEDING
        ))::bigint AS balance
    FROM days d
    CROSS JOIN base_balance bb
    LEFT JOIN daily_totals dt ON dt.day = d.day
)
SELECT COALESCE(AVG(balance), 0)::bigint AS avg_balance
FROM daily_balances
            "#,
        )
        .bind(account_id)
        .bind(start_date)
        .bind(effective_end)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map_or(0, |r| r.avg_balance))
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
            top_up_amount: None,
            top_up_cycle: None,
            top_up_day: None,
            statement_close_day: None,
            payment_due_day: None,
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
            top_up_amount: None,
            top_up_cycle: None,
            top_up_day: None,
            statement_close_day: None,
            payment_due_day: None,
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
            top_up_amount: None,
            top_up_cycle: None,
            top_up_day: None,
            statement_close_day: None,
            payment_due_day: None,
        };
        assert_eq!(account_type_to_db(&request_credit.account_type), "CreditCard");
    }
}
