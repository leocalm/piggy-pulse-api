use crate::database::postgres_repository::{PostgresRepository, is_unique_violation};
use crate::error::app_error::AppError;
use crate::models::account::{
    Account, AccountBalanceHistoryPoint, AccountContextResponse, AccountDetailResponse, AccountRequest, AccountStability, AccountTransactionResponse,
    AccountType, AccountUpdateRequest, AccountWithMetrics, CategoryImpactItem,
};
use crate::models::currency::{Currency, SymbolPosition};
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
        // current_balance comes from account_balance_state (maintained by the
        // aggregate trigger). transaction_count is the total all-time count
        // of distinct effective logical transactions touching this account.
        // balance_change_this_period is left at 0 here — this endpoint is
        // not period-scoped; callers that need it use list_accounts.
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
                (a.balance + COALESCE(abs.sum_amount, 0))::bigint AS current_balance,
                0::bigint AS balance_change_this_period,
                COALESCE(abs.tx_count, 0)::bigint AS transaction_count
            FROM account a
            JOIN currency c ON c.id = a.currency_id
            LEFT JOIN account_balance_state abs ON abs.account_id = a.id
            WHERE a.id = $1 AND a.user_id = $2
            "#,
        )
        .bind(id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(AccountWithMetrics::from))
    }

    pub async fn list_accounts(&self, params: &CursorParams, budget_period_id: &Uuid, user_id: &Uuid) -> Result<Vec<AccountWithMetrics>, AppError> {
        // current_balance comes from account_balance_state; the period change
        // and period transaction count come from account_daily_delta summed
        // over the period's day range. Neither read touches the `transaction`
        // table.
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
                    (a.balance + COALESCE(abs.sum_amount, 0))::bigint AS current_balance,
                    COALESCE((
                        SELECT SUM(add.inflow - add.outflow)::bigint
                          FROM account_daily_delta add, period p
                         WHERE add.account_id = a.id
                           AND add.day BETWEEN p.start_date AND p.end_date
                    ), 0)::bigint AS balance_change_this_period,
                    COALESCE((
                        SELECT SUM(add.tx_count)::bigint
                          FROM account_daily_delta add, period p
                         WHERE add.account_id = a.id
                           AND add.day BETWEEN p.start_date AND p.end_date
                    ), 0)::bigint AS transaction_count
                FROM account a
                JOIN currency c ON c.id = a.currency_id
                LEFT JOIN account_balance_state abs ON abs.account_id = a.id
                WHERE (a.created_at, a.id) < (
                    SELECT created_at, id FROM account WHERE id = $1
                ) AND a.user_id = $3 AND a.is_archived = FALSE
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
                    (a.balance + COALESCE(abs.sum_amount, 0))::bigint AS current_balance,
                    COALESCE((
                        SELECT SUM(add.inflow - add.outflow)::bigint
                          FROM account_daily_delta add, period p
                         WHERE add.account_id = a.id
                           AND add.day BETWEEN p.start_date AND p.end_date
                    ), 0)::bigint AS balance_change_this_period,
                    COALESCE((
                        SELECT SUM(add.tx_count)::bigint
                          FROM account_daily_delta add, period p
                         WHERE add.account_id = a.id
                           AND add.day BETWEEN p.start_date AND p.end_date
                    ), 0)::bigint AS transaction_count
                FROM account a
                JOIN currency c ON c.id = a.currency_id
                LEFT JOIN account_balance_state abs ON abs.account_id = a.id
                WHERE a.user_id = $2 AND a.is_archived = FALSE
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

        // We re-fetch the existing account to get the current currency and balance
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

        // Build the update query conditionally based on whether balance is provided
        let (sql, balance_to_use) = if let Some(account_balance) = request.balance {
            (
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
                account_balance,
            )
        } else {
            (
                r#"
                UPDATE account
                SET name = $1, color = $2, icon = $3, account_type = $4::text::account_type, balance = balance, spend_limit = $6, next_transfer_amount = $7,
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
                existing_account.balance, // Use existing balance as dummy bind value
            )
        };

        let row = sqlx::query_as::<_, UpdateAccountRow>(sql)
            .bind(&request.name)
            .bind(&request.color)
            .bind(&request.icon)
            .bind(&account_type_str)
            .bind(balance_to_use)
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

        // All metrics come from account_daily_delta summed over the period.
        // `inflow`/`outflow` on account_daily_delta already follow the
        // from/to semantics used by the legacy query (Incoming credits the
        // from account; Outgoing/Transfer-out debit it; Transfer-in credits
        // the to account), so a raw sum matches the legacy classification.
        let row = sqlx::query_as::<_, DetailRow>(
            r#"
SELECT
    a.balance                              AS balance,
    COALESCE(SUM(add1.inflow), 0)::bigint  AS inflows,
    COALESCE(SUM(add1.outflow), 0)::bigint AS outflows,
    COALESCE(SUM(add1.tx_count), 0)::bigint AS transaction_count,
    bp.start_date                          AS period_start,
    bp.end_date                            AS period_end
FROM account a
CROSS JOIN budget_period bp
LEFT JOIN account_daily_delta add1
       ON add1.account_id = a.id
      AND add1.day BETWEEN bp.start_date AND bp.end_date
WHERE a.id = $1 AND a.user_id = $3
  AND bp.id = $2 AND bp.user_id = $3
GROUP BY a.balance, bp.start_date, bp.end_date
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

        // Compute per-day running balance from account_daily_delta:
        //   base_balance = account.balance + account_balance_state.sum_amount
        //                  − SUM(account_daily_delta after start_date)
        // Then layer daily (inflow − outflow) running-sum over the date range.
        let rows = sqlx::query_as::<_, HistoryRow>(
            r#"
WITH days AS (
    SELECT d::date AS day
    FROM generate_series($2::date, $3::date, '1 day') AS d
),
base_balance AS (
    SELECT
        a.balance
        + COALESCE(abs.sum_amount, 0)
        - COALESCE((
            SELECT SUM(add1.inflow - add1.outflow)
              FROM account_daily_delta add1
             WHERE add1.account_id = a.id
               AND add1.day >= $2
        ), 0) AS base_bal
    FROM account a
    LEFT JOIN account_balance_state abs ON abs.account_id = a.id
    WHERE a.id = $1 AND a.user_id = $4
),
daily_totals AS (
    SELECT add1.day,
           (add1.inflow - add1.outflow)::bigint AS daily_amount,
           add1.tx_count::bigint                AS tx_count
    FROM account_daily_delta add1
    WHERE add1.account_id = $1
      AND add1.day BETWEEN $2 AND $3
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

        // Cursor resolves the UUID to (occurred_at, id) via Latest_Row lookup
        // on logical_transaction_state.
        let (cursor_clause, has_cursor) = if params.cursor.is_some() {
            (
                "AND (occurred_at, id) < ( \
                    SELECT t2.occurred_at, lts2.id \
                      FROM logical_transaction_state lts2 \
                      JOIN transaction t2 ON t2.id = lts2.id AND t2.seq = lts2.latest_seq \
                     WHERE lts2.id = $5 AND lts2.user_id = $4)",
                true,
            )
        } else {
            ("", false)
        };

        // Base balance = account balance + all-time aggregate sum −
        //                 (sum of account_daily_delta on/after period start).
        // period_txs joins logical_transaction_state so only Latest_Rows of
        // effective logical transactions appear — voided and reversal rows
        // are invisible.
        let sql = format!(
            r#"
WITH period AS (
    SELECT start_date, end_date FROM budget_period WHERE id = $2 AND user_id = $4
),
base_balance AS (
    SELECT
        a.balance
        + COALESCE(abs.sum_amount, 0)
        - COALESCE((
            SELECT SUM(add1.inflow - add1.outflow)
              FROM account_daily_delta add1
             WHERE add1.account_id = a.id
               AND add1.day >= (SELECT start_date FROM period)
        ), 0) AS base_bal
    FROM account a
    LEFT JOIN account_balance_state abs ON abs.account_id = a.id
    WHERE a.id = $1 AND a.user_id = $4
),
period_txs AS (
    SELECT
        lts.id,
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
    FROM logical_transaction_state lts
    JOIN transaction t ON t.id = lts.id AND t.seq = lts.latest_seq
    JOIN category cat ON cat.id = t.category_id
    CROSS JOIN period
    WHERE (t.from_account_id = $1 OR t.to_account_id = $1)
      AND lts.user_id = $4
      AND lts.is_effective
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

        // Account top categories in period is one of the three sanctioned
        // ledger scans (Req 12 AC 5 + design open-question #2). There is no
        // (account, category, day) aggregate table, so we scan the
        // Latest_Rows joined to logical_transaction_state over the narrow
        // account+period slice. Acceptable because the account detail page
        // is reached explicitly and the slice is small.
        let cat_rows = sqlx::query_as::<_, CategoryRow>(
            r#"
WITH period AS (
    SELECT start_date, end_date FROM budget_period WHERE id = $2 AND user_id = $3
)
SELECT
    cat.id AS category_id,
    cat.name AS category_name,
    SUM(t.amount)::bigint AS amount
FROM logical_transaction_state lts
JOIN transaction t ON t.id = lts.id AND t.seq = lts.latest_seq
JOIN category cat ON cat.id = t.category_id
CROSS JOIN period
WHERE t.from_account_id = $1
  AND lts.user_id = $3
  AND lts.is_effective
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

        // Net flow per closed period comes from account_daily_delta summed
        // over each period's day range. The base balance is the account's
        // current balance (including all-time aggregate), matching the
        // legacy computation.
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
        COALESCE((
            SELECT SUM(add1.inflow - add1.outflow)
              FROM account_daily_delta add1
             WHERE add1.account_id = $1
               AND add1.day BETWEEN pp.start_date AND pp.end_date
        ), 0)::bigint AS net_flow
    FROM prior_periods pp
),
base AS (
    SELECT (a.balance + COALESCE(abs.sum_amount, 0))::bigint AS current_balance
      FROM account a
      LEFT JOIN account_balance_state abs ON abs.account_id = a.id
     WHERE a.id = $1 AND a.user_id = $2
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

        // Narrow ledger scan joined to logical_transaction_state so only
        // Latest_Rows of effective logical transactions are considered. This
        // is a single-row query over a bounded date range per account — no
        // aggregate table fits this "largest single outflow" use case.
        let largest = sqlx::query_as::<_, OutflowRow>(
            r#"
WITH period AS (
    SELECT start_date, end_date FROM budget_period WHERE id = $2 AND user_id = $3
)
SELECT
    cat.name AS category_name,
    t.amount::bigint AS amount
FROM logical_transaction_state lts
JOIN transaction t ON t.id = lts.id AND t.seq = lts.latest_seq
JOIN category cat ON cat.id = t.category_id
CROSS JOIN period
WHERE t.from_account_id = $1
  AND lts.user_id = $3
  AND lts.is_effective
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

    pub async fn list_active_account_ids(&self, user_id: &Uuid) -> Result<Vec<Uuid>, AppError> {
        let ids: Vec<Uuid> = sqlx::query_scalar("SELECT id FROM account WHERE user_id = $1 AND is_archived = FALSE ORDER BY created_at DESC")
            .bind(user_id)
            .fetch_all(&self.pool)
            .await?;
        Ok(ids)
    }

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
                    (a.balance + COALESCE(abs.sum_amount, 0))::bigint AS current_balance,
                    0::bigint AS balance_change_this_period,
                    COALESCE(abs.tx_count, 0)::bigint AS transaction_count
                FROM account a
                JOIN currency c ON c.id = a.currency_id
                LEFT JOIN account_balance_state abs ON abs.account_id = a.id
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
                    (a.balance + COALESCE(abs.sum_amount, 0))::bigint AS current_balance,
                    0::bigint AS balance_change_this_period,
                    COALESCE(abs.tx_count, 0)::bigint AS transaction_count
                FROM account a
                JOIN currency c ON c.id = a.currency_id
                LEFT JOIN account_balance_state abs ON abs.account_id = a.id
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

        Ok((rows.into_iter().map(AccountWithMetrics::from).collect(), total_count))
    }

    /// Compute the total outgoing spend for an Allowance account since the start of its current
    /// top-up cycle.  Returns 0 if the account has no top-up configuration or no recognised cycle.
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
            -- bi-weekly: find the most-recent matching weekday, then go back an extra 7 days
            -- if that anchor falls within the current 7-day window (i.e. we are still in the first
            -- week of the cycle) so that we always cover a full 14-day window.
            WHEN 'bi-weekly' THEN (
                WITH weekly_anchor AS (
                    SELECT CURRENT_DATE
                        - ((EXTRACT(DOW FROM CURRENT_DATE)::int - cfg.top_up_day + 7) % 7) * INTERVAL '1 day'
                        AS anchor
                )
                SELECT
                    CASE
                        WHEN (CURRENT_DATE - anchor::date) < 7 THEN anchor::date - INTERVAL '7 days'
                        ELSE anchor::date
                    END
                FROM weekly_anchor
            )
            -- monthly: day cfg.top_up_day of the current month (or last month if not yet reached)
            WHEN 'monthly' THEN
                CASE
                    WHEN EXTRACT(DAY FROM CURRENT_DATE)::int >= cfg.top_up_day
                    THEN DATE_TRUNC('month', CURRENT_DATE) + (cfg.top_up_day - 1) * INTERVAL '1 day'
                    ELSE DATE_TRUNC('month', CURRENT_DATE - INTERVAL '1 month') + (cfg.top_up_day - 1) * INTERVAL '1 day'
                END
            -- unknown / NULL cycle: return NULL so no transactions are matched
            ELSE NULL
        END::date AS start_date
    FROM account_cfg cfg
)
-- Allowance spending is: money going OUT of this account (Outgoing
-- purchases + outgoing Transfers). `account_daily_delta.outflow` already
-- captures both classifications via the from-side delta.
SELECT
    COALESCE(SUM(add1.outflow), 0)::bigint AS spent
FROM cycle_start cs
LEFT JOIN account_daily_delta add1
    ON add1.account_id = $1
    AND cs.start_date IS NOT NULL
    AND add1.day >= cs.start_date
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

        // Mirrors get_account_balance_history but averages the daily balances
        // instead of returning them one by one.
        let row = sqlx::query_as::<_, AvgRow>(
            r#"
WITH days AS (
    SELECT d::date AS day
    FROM generate_series($2::date, $3::date, '1 day') AS d
),
base_balance AS (
    SELECT
        a.balance
        + COALESCE(abs.sum_amount, 0)
        - COALESCE((
            SELECT SUM(add1.inflow - add1.outflow)
              FROM account_daily_delta add1
             WHERE add1.account_id = a.id
               AND add1.day >= $2
        ), 0) AS base_bal
    FROM account a
    LEFT JOIN account_balance_state abs ON abs.account_id = a.id
    WHERE a.id = $1 AND a.user_id = $4
),
daily_totals AS (
    SELECT add1.day,
           (add1.inflow - add1.outflow)::bigint AS daily_amount
    FROM account_daily_delta add1
    WHERE add1.account_id = $1
      AND add1.day BETWEEN $2 AND $3
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
