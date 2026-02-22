use crate::database::category::category_type_from_db;
use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::models::account::Account;
use crate::models::category::{Category, CategoryType};
use crate::models::currency::{Currency, SymbolPosition};
use crate::models::pagination::{CursorParams, TransactionFilters};
use crate::models::transaction::{Transaction, TransactionRequest};
use crate::models::transaction_summary::TransactionSummary;
use crate::models::vendor::Vendor;
use chrono::NaiveDate;
use uuid::Uuid;

// Intermediate struct for sqlx query results with all JOINed data
#[derive(Debug, sqlx::FromRow)]
struct TransactionRow {
    id: Uuid,
    amount: i64,
    description: String,
    occurred_at: NaiveDate,
    // Category fields
    category_id: Uuid,
    category_name: String,
    category_color: String,
    category_icon: String,
    category_parent_id: Option<Uuid>,
    category_category_type: String,
    category_is_archived: bool,
    category_description: Option<String>,
    // From account fields
    from_account_id: Uuid,
    from_account_name: String,
    from_account_color: String,
    from_account_icon: String,
    from_account_account_type: String,
    from_account_balance: i64,
    from_account_spend_limit: Option<i32>,
    from_account_currency_id: Uuid,
    from_account_currency_name: String,
    from_account_currency_symbol: String,
    from_account_currency_code: String,
    from_account_currency_decimal_places: i32,
    from_account_currency_symbol_position: SymbolPosition,
    // To account fields (optional)
    to_account_id: Option<Uuid>,
    to_account_name: Option<String>,
    to_account_color: Option<String>,
    to_account_icon: Option<String>,
    to_account_account_type: Option<String>,
    to_account_balance: Option<i64>,
    to_account_spend_limit: Option<i32>,
    to_account_currency_id: Option<Uuid>,
    to_account_currency_name: Option<String>,
    to_account_currency_symbol: Option<String>,
    to_account_currency_code: Option<String>,
    to_account_currency_decimal_places: Option<i32>,
    to_account_currency_symbol_position: Option<SymbolPosition>,
    // Vendor fields (optional)
    vendor_id: Option<Uuid>,
    vendor_name: Option<String>,
    vendor_description: Option<String>,
    vendor_archived: Option<bool>,
}

impl From<TransactionRow> for Transaction {
    fn from(row: TransactionRow) -> Self {
        let to_account = if let Some(to_account_id) = row.to_account_id {
            Some(Account {
                id: to_account_id,
                name: row.to_account_name.unwrap(),
                color: row.to_account_color.unwrap(),
                icon: row.to_account_icon.unwrap(),
                account_type: crate::database::account::account_type_from_db(row.to_account_account_type.unwrap()),
                currency: Currency {
                    id: row.to_account_currency_id.unwrap(),
                    name: row.to_account_currency_name.unwrap(),
                    symbol: row.to_account_currency_symbol.unwrap(),
                    currency: row.to_account_currency_code.unwrap(),
                    decimal_places: row.to_account_currency_decimal_places.unwrap(),
                    symbol_position: row.to_account_currency_symbol_position.unwrap(),
                },
                balance: row.to_account_balance.unwrap(),
                spend_limit: row.to_account_spend_limit,
                is_archived: false,
                next_transfer_amount: None,
            })
        } else {
            None
        };

        let vendor = if let Some(vendor_id) = row.vendor_id {
            Some(Vendor {
                id: vendor_id,
                name: row.vendor_name.unwrap(),
                description: row.vendor_description,
                archived: row.vendor_archived.unwrap_or(false),
            })
        } else {
            None
        };

        Transaction {
            id: row.id,
            amount: row.amount,
            description: row.description,
            occurred_at: row.occurred_at,
            category: Category {
                id: row.category_id,
                name: row.category_name,
                color: row.category_color,
                icon: row.category_icon,
                parent_id: row.category_parent_id,
                category_type: category_type_from_db(&row.category_category_type),
                is_archived: row.category_is_archived,
                description: row.category_description,
            },

            from_account: Account {
                id: row.from_account_id,
                name: row.from_account_name,
                color: row.from_account_color,
                icon: row.from_account_icon,
                account_type: crate::database::account::account_type_from_db(&row.from_account_account_type),
                currency: Currency {
                    id: row.from_account_currency_id,
                    name: row.from_account_currency_name,
                    symbol: row.from_account_currency_symbol,
                    currency: row.from_account_currency_code,
                    decimal_places: row.from_account_currency_decimal_places,
                    symbol_position: row.from_account_currency_symbol_position,
                },
                balance: row.from_account_balance,
                spend_limit: row.from_account_spend_limit,
                is_archived: false,
                next_transfer_amount: None,
            },

            to_account,
            vendor,
        }
    }
}

// Common SELECT clause for transaction queries with all joined data
const TRANSACTION_SELECT_FIELDS: &str = r#"
    t.id,
    t.amount,
    t.description,
    t.occurred_at,
    c.id as category_id,
    c.name as category_name,
    COALESCE(c.color, '') as category_color,
    COALESCE(c.icon, '') as category_icon,
    c.parent_id as category_parent_id,
    c.category_type::text as category_category_type,
    c.is_archived as category_is_archived,
    c.description as category_description,
    fa.id as from_account_id,
    fa.name as from_account_name,
    fa.color as from_account_color,
    fa.icon as from_account_icon,
    fa.account_type::text as from_account_account_type,
    fa.balance as from_account_balance,
    fa.spend_limit as from_account_spend_limit,
    cfa.id as from_account_currency_id,
    cfa.name as from_account_currency_name,
    cfa.symbol as from_account_currency_symbol,
    cfa.currency as from_account_currency_code,
    cfa.decimal_places as from_account_currency_decimal_places,
    cfa.symbol_position as from_account_currency_symbol_position,
    ta.id as to_account_id,
    ta.name as to_account_name,
    ta.color as to_account_color,
    ta.icon as to_account_icon,
    ta.account_type::text as to_account_account_type,
    ta.balance as to_account_balance,
    ta.spend_limit as to_account_spend_limit,
    cta.id as to_account_currency_id,
    cta.name as to_account_currency_name,
    cta.symbol as to_account_currency_symbol,
    cta.currency as to_account_currency_code,
    cta.decimal_places as to_account_currency_decimal_places,
    cta.symbol_position as to_account_currency_symbol_position,
    v.id as vendor_id,
    v.name as vendor_name
"#;

// Common JOIN clauses for transaction queries
const TRANSACTION_JOINS: &str = r#"
    JOIN category c ON t.category_id = c.id
    JOIN account fa ON t.from_account_id = fa.id
    JOIN currency cfa ON fa.currency_id = cfa.id
    LEFT JOIN account ta ON t.to_account_id = ta.id
    LEFT JOIN currency cta ON ta.currency_id = cta.id
    LEFT JOIN vendor v ON t.vendor_id = v.id
"#;

/// Builds a complete SELECT query for transactions with the specified table/CTE name and WHERE clauses
fn build_transaction_query(from_clause: &str, base_where: &str, extra_where: &str, order_by: &str) -> String {
    let mut query = format!("SELECT {} FROM {} {}", TRANSACTION_SELECT_FIELDS, from_clause, TRANSACTION_JOINS);

    let clauses: Vec<&str> = [base_where, extra_where]
        .iter()
        .filter(|s| !s.is_empty())
        .copied()
        .collect();

    if !clauses.is_empty() {
        query.push_str("WHERE ");
        query.push_str(&clauses.join(" AND "));
    }

    if !order_by.is_empty() {
        query.push_str(" ORDER BY ");
        query.push_str(order_by);
    }

    query
}

/// Enum for filter bind values to enable dynamic binding
#[derive(Debug)]
enum FilterBindValue {
    UuidArray(Vec<Uuid>),
    Text(String),
    Date(NaiveDate),
}

/// Builds additional WHERE fragments and collects bind values for TransactionFilters.
/// Returns (sql_fragment, bind_values) where bind values should be bound in order.
fn build_filter_clause(filters: &TransactionFilters, start_offset: usize) -> (String, Vec<FilterBindValue>) {
    let mut parts = Vec::new();
    let mut binds: Vec<FilterBindValue> = Vec::new();
    let mut n = start_offset;

    if !filters.account_ids.is_empty() {
        parts.push(format!("t.from_account_id = ANY(${})", n));
        binds.push(FilterBindValue::UuidArray(filters.account_ids.clone()));
        n += 1;
    }
    if !filters.category_ids.is_empty() {
        parts.push(format!("t.category_id = ANY(${})", n));
        binds.push(FilterBindValue::UuidArray(filters.category_ids.clone()));
        n += 1;
    }
    if let Some(ref dir) = filters.direction {
        parts.push(format!("c.category_type::text = ${}", n));
        binds.push(FilterBindValue::Text(dir.clone()));
        n += 1;
    }
    if !filters.vendor_ids.is_empty() {
        parts.push(format!("t.vendor_id = ANY(${})", n));
        binds.push(FilterBindValue::UuidArray(filters.vendor_ids.clone()));
        n += 1;
    }
    if let Some(date_from) = filters.date_from {
        parts.push(format!("t.occurred_at >= ${}", n));
        binds.push(FilterBindValue::Date(date_from));
        n += 1;
    }
    if let Some(date_to) = filters.date_to {
        parts.push(format!("t.occurred_at <= ${}", n));
        binds.push(FilterBindValue::Date(date_to));
        n += 1;
    }
    let _ = n; // suppress unused warning

    (parts.join(" AND "), binds)
}

/// Binds a filter value to a query
fn bind_filter_value<'q>(
    q: sqlx::query::QueryAs<'q, sqlx::Postgres, TransactionRow, sqlx::postgres::PgArguments>,
    bind: &'q FilterBindValue,
) -> sqlx::query::QueryAs<'q, sqlx::Postgres, TransactionRow, sqlx::postgres::PgArguments> {
    match bind {
        FilterBindValue::UuidArray(ids) => q.bind(ids),
        FilterBindValue::Text(s) => q.bind(s),
        FilterBindValue::Date(d) => q.bind(d),
    }
}

impl PostgresRepository {
    async fn validate_transaction_ownership(&self, transaction: &TransactionRequest, user_id: &Uuid) -> Result<(), AppError> {
        let category_exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM category WHERE id = $1 AND user_id = $2)")
            .bind(transaction.category_id)
            .bind(user_id)
            .fetch_one(&self.pool)
            .await?;
        if !category_exists {
            return Err(AppError::BadRequest("Invalid category_id for current user".to_string()));
        }

        let from_account_exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM account WHERE id = $1 AND user_id = $2)")
            .bind(transaction.from_account_id)
            .bind(user_id)
            .fetch_one(&self.pool)
            .await?;
        if !from_account_exists {
            return Err(AppError::BadRequest("Invalid from_account_id for current user".to_string()));
        }

        if let Some(to_account_id) = transaction.to_account_id {
            let to_account_exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM account WHERE id = $1 AND user_id = $2)")
                .bind(to_account_id)
                .bind(user_id)
                .fetch_one(&self.pool)
                .await?;
            if !to_account_exists {
                return Err(AppError::BadRequest("Invalid to_account_id for current user".to_string()));
            }
        }

        if let Some(vendor_id) = transaction.vendor_id {
            let vendor_exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM vendor WHERE id = $1 AND user_id = $2)")
                .bind(vendor_id)
                .bind(user_id)
                .fetch_one(&self.pool)
                .await?;
            if !vendor_exists {
                return Err(AppError::BadRequest("Invalid vendor_id for current user".to_string()));
            }
        }

        Ok(())
    }

    pub async fn create_transaction(&self, transaction: &TransactionRequest, user_id: &Uuid) -> Result<Transaction, AppError> {
        self.validate_transaction_ownership(transaction, user_id).await?;

        let to_account_id = if let Some(acc_id) = &transaction.to_account_id { Some(acc_id) } else { None };
        let vendor_id = if let Some(v_id) = &transaction.vendor_id { Some(v_id) } else { None };

        let select_query = build_transaction_query("inserted t", "", "", "");
        let query = format!(
            r#"
            WITH inserted AS (
                INSERT INTO transaction (
                    user_id,
                    amount,
                    description,
                    occurred_at,
                    category_id,
                    from_account_id,
                    to_account_id,
                    vendor_id
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                RETURNING id, amount, description, occurred_at, category_id, from_account_id, to_account_id, vendor_id
            )
            {}
            "#,
            select_query
        );

        let row = sqlx::query_as::<_, TransactionRow>(&query)
            .bind(user_id)
            .bind(transaction.amount)
            .bind(&transaction.description)
            .bind(transaction.occurred_at)
            .bind(transaction.category_id)
            .bind(transaction.from_account_id)
            .bind(to_account_id)
            .bind(vendor_id)
            .fetch_one(&self.pool)
            .await?;

        Ok(Transaction::from(row))
    }

    pub async fn get_transaction_by_id(&self, id: &Uuid, user_id: &Uuid) -> Result<Option<Transaction>, AppError> {
        let query = build_transaction_query("transaction t", "t.id = $1 AND t.user_id = $2", "", "");
        let row = sqlx::query_as::<_, TransactionRow>(&query)
            .bind(id)
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.map(Transaction::from))
    }

    pub async fn list_transactions(&self, params: &CursorParams, filters: &TransactionFilters, user_id: &Uuid) -> Result<Vec<Transaction>, AppError> {
        let rows = if let Some(cursor) = params.cursor {
            let (filter_sql, filter_binds) = build_filter_clause(filters, 3); // $1=user_id, $2=cursor
            let base = build_transaction_query(
                "transaction t",
                "t.user_id = $1 AND (t.occurred_at, t.created_at, t.id) < (SELECT occurred_at, created_at, id FROM transaction WHERE id = $2)",
                &filter_sql,
                "t.occurred_at DESC, t.created_at DESC, t.id DESC",
            );
            let limit_n = 3 + filter_binds.len();
            let full_query = format!("{} LIMIT ${}", base, limit_n);
            let mut q = sqlx::query_as::<_, TransactionRow>(&full_query)
                .bind(user_id)
                .bind(cursor);
            for bind in &filter_binds {
                q = bind_filter_value(q, bind);
            }
            q.bind(params.fetch_limit()).fetch_all(&self.pool).await?
        } else {
            let (filter_sql, filter_binds) = build_filter_clause(filters, 2); // $1=user_id
            let base = build_transaction_query(
                "transaction t",
                "t.user_id = $1",
                &filter_sql,
                "t.occurred_at DESC, t.created_at DESC, t.id DESC"
            );
            let limit_n = 2 + filter_binds.len();
            let full_query = format!("{} LIMIT ${}", base, limit_n);
            let mut q = sqlx::query_as::<_, TransactionRow>(&full_query).bind(user_id);
            for bind in &filter_binds {
                q = bind_filter_value(q, bind);
            }
            q.bind(params.fetch_limit()).fetch_all(&self.pool).await?
        };

        Ok(rows.into_iter().map(Transaction::from).collect())
    }

    pub async fn get_transactions_for_period(&self, period_id: &Uuid, params: &CursorParams, filters: &TransactionFilters, user_id: &Uuid) -> Result<Vec<Transaction>, AppError> {
        let rows = if let Some(cursor) = params.cursor {
            let (filter_sql, filter_binds) = build_filter_clause(filters, 4); // $1=period_id, $2=user_id, $3=cursor
            let base_where = "bp.id = $1 \
                   AND bp.user_id = $2 \
                   AND t.user_id = $2 \
                   AND t.occurred_at >= bp.start_date \
                   AND t.occurred_at <= bp.end_date \
                   AND (t.occurred_at, t.created_at, t.id) < (SELECT occurred_at, created_at, id FROM transaction WHERE id = $3)";
            let combined_where = if filter_sql.is_empty() {
                base_where.to_string()
            } else {
                format!("{} AND {}", base_where, filter_sql)
            };
            let query = format!(
                "SELECT {} FROM transaction t CROSS JOIN budget_period bp {} \
                 WHERE {} \
                 ORDER BY t.occurred_at DESC, t.created_at DESC, t.id DESC \
                 LIMIT ${}",
                TRANSACTION_SELECT_FIELDS, TRANSACTION_JOINS, combined_where, 4 + filter_binds.len()
            );
            let mut q = sqlx::query_as::<_, TransactionRow>(&query)
                .bind(period_id)
                .bind(user_id)
                .bind(cursor);
            for bind in &filter_binds {
                q = bind_filter_value(q, bind);
            }
            q.bind(params.fetch_limit()).fetch_all(&self.pool).await?
        } else {
            let (filter_sql, filter_binds) = build_filter_clause(filters, 3); // $1=period_id, $2=user_id
            let base_where = "bp.id = $1 \
                   AND bp.user_id = $2 \
                   AND t.user_id = $2 \
                   AND t.occurred_at >= bp.start_date \
                   AND t.occurred_at <= bp.end_date";
            let combined_where = if filter_sql.is_empty() {
                base_where.to_string()
            } else {
                format!("{} AND {}", base_where, filter_sql)
            };
            let query = format!(
                "SELECT {} FROM transaction t CROSS JOIN budget_period bp {} \
                 WHERE {} \
                 ORDER BY t.occurred_at DESC, t.created_at DESC, t.id DESC \
                 LIMIT ${}",
                TRANSACTION_SELECT_FIELDS, TRANSACTION_JOINS, combined_where, 3 + filter_binds.len()
            );
            let mut q = sqlx::query_as::<_, TransactionRow>(&query)
                .bind(period_id)
                .bind(user_id);
            for bind in &filter_binds {
                q = bind_filter_value(q, bind);
            }
            q.bind(params.fetch_limit()).fetch_all(&self.pool).await?
        };

        Ok(rows.into_iter().map(Transaction::from).collect())
    }

    pub async fn delete_transaction(&self, id: &Uuid, user_id: &Uuid) -> Result<(), AppError> {
        sqlx::query("DELETE FROM transaction WHERE id = $1 AND user_id = $2")
            .bind(id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn update_transaction(&self, id: &Uuid, transaction: &TransactionRequest, user_id: &Uuid) -> Result<Transaction, AppError> {
        self.validate_transaction_ownership(transaction, user_id).await?;

        let select_query = build_transaction_query("updated t", "", "", "");
        let query = format!(
            r#"
            WITH updated AS (
                UPDATE transaction
                SET
                    amount = $1,
                    description = $2,
                    occurred_at = $3,
                    category_id = $4,
                    from_account_id = $5,
                    to_account_id = $6,
                    vendor_id = $7
                WHERE id = $8 AND user_id = $9
                RETURNING id, amount, description, occurred_at, category_id, from_account_id, to_account_id, vendor_id
            )
            {}
            "#,
            select_query
        );

        let row = sqlx::query_as::<_, TransactionRow>(&query)
            .bind(transaction.amount)
            .bind(&transaction.description)
            .bind(transaction.occurred_at)
            .bind(transaction.category_id)
            .bind(transaction.from_account_id)
            .bind(transaction.to_account_id)
            .bind(transaction.vendor_id)
            .bind(id)
            .bind(user_id)
            .fetch_one(&self.pool)
            .await?;

        Ok(Transaction::from(row))
    }

    pub async fn get_transaction_summary(&self, period_id: &Uuid, user_id: &Uuid) -> Result<TransactionSummary, AppError> {
        #[derive(sqlx::FromRow)]
        struct SummaryRow {
            category_type: String,
            total_amount: i64,
        }

        let rows = sqlx::query_as::<_, SummaryRow>(
            r#"
            SELECT c.category_type::text, COALESCE(SUM(t.amount), 0) as total_amount
            FROM transaction t
            JOIN category c ON t.category_id = c.id
            CROSS JOIN budget_period bp
            WHERE bp.id = $1
                AND bp.user_id = $2
                AND t.user_id = $2
                AND t.occurred_at >= bp.start_date
                AND t.occurred_at <= bp.end_date
            GROUP BY c.category_type
            "#,
        )
        .bind(period_id)
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        let mut total_income = 0i32;
        let mut total_expense = 0i32;

        for row in rows {
            let category_type = category_type_from_db(&row.category_type);
            let amount = row.total_amount as i32;

            match category_type {
                CategoryType::Incoming => total_income += amount,
                CategoryType::Outgoing => total_expense += amount,
                CategoryType::Transfer => {} // Transfers are not counted in summary
            }
        }

        let net_difference = total_income - total_expense;

        Ok(TransactionSummary {
            total_income,
            total_expense,
            net_difference,
        })
    }
}
