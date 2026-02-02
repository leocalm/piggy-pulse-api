use crate::database::category::category_type_from_db;
use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::models::account::Account;
use crate::models::category::Category;
use crate::models::currency::Currency;
use crate::models::pagination::PaginationParams;
use crate::models::transaction::{Transaction, TransactionRequest};
use crate::models::vendor::Vendor;
use chrono::{DateTime, NaiveDate, Utc};
use uuid::Uuid;

// Intermediate struct for sqlx query results with all JOINed data
#[derive(Debug, sqlx::FromRow)]
struct TransactionRow {
    id: Uuid,
    amount: i32,
    description: String,
    occurred_at: NaiveDate,
    // Category fields
    category_id: Uuid,
    category_name: String,
    category_color: String,
    category_icon: String,
    category_parent_id: Option<Uuid>,
    category_category_type: String,
    category_created_at: DateTime<Utc>,
    // From account fields
    from_account_id: Uuid,
    from_account_name: String,
    from_account_color: String,
    from_account_icon: String,
    from_account_account_type: String,
    from_account_balance: i64,
    from_account_created_at: DateTime<Utc>,
    from_account_spend_limit: Option<i32>,
    from_account_currency_id: Uuid,
    from_account_currency_name: String,
    from_account_currency_symbol: String,
    from_account_currency_code: String,
    from_account_currency_decimal_places: i32,
    from_account_currency_created_at: DateTime<Utc>,
    // To account fields (optional)
    to_account_id: Option<Uuid>,
    to_account_name: Option<String>,
    to_account_color: Option<String>,
    to_account_icon: Option<String>,
    to_account_account_type: Option<String>,
    to_account_balance: Option<i64>,
    to_account_created_at: Option<DateTime<Utc>>,
    to_account_spend_limit: Option<i32>,
    to_account_currency_id: Option<Uuid>,
    to_account_currency_name: Option<String>,
    to_account_currency_symbol: Option<String>,
    to_account_currency_code: Option<String>,
    to_account_currency_decimal_places: Option<i32>,
    to_account_currency_created_at: Option<DateTime<Utc>>,
    // Vendor fields (optional)
    vendor_id: Option<Uuid>,
    vendor_name: Option<String>,
    vendor_created_at: Option<DateTime<Utc>>,
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
                    created_at: row.to_account_currency_created_at.unwrap(),
                },
                balance: row.to_account_balance.unwrap(),
                created_at: row.to_account_created_at.unwrap(),
                spend_limit: row.to_account_spend_limit,
            })
        } else {
            None
        };

        let vendor = if let Some(vendor_id) = row.vendor_id {
            Some(Vendor {
                id: vendor_id,
                name: row.vendor_name.unwrap(),
                created_at: row.vendor_created_at.unwrap(),
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
                created_at: row.category_created_at,
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
                    created_at: row.from_account_currency_created_at,
                },
                balance: row.from_account_balance,
                created_at: row.from_account_created_at,
                spend_limit: row.from_account_spend_limit,
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
    c.created_at as category_created_at,
    fa.id as from_account_id,
    fa.name as from_account_name,
    fa.color as from_account_color,
    fa.icon as from_account_icon,
    fa.account_type::text as from_account_account_type,
    fa.balance as from_account_balance,
    fa.created_at as from_account_created_at,
    fa.spend_limit as from_account_spend_limit,
    cfa.id as from_account_currency_id,
    cfa.name as from_account_currency_name,
    cfa.symbol as from_account_currency_symbol,
    cfa.currency as from_account_currency_code,
    cfa.decimal_places as from_account_currency_decimal_places,
    cfa.created_at as from_account_currency_created_at,
    ta.id as to_account_id,
    ta.name as to_account_name,
    ta.color as to_account_color,
    ta.icon as to_account_icon,
    ta.account_type::text as to_account_account_type,
    ta.balance as to_account_balance,
    ta.created_at as to_account_created_at,
    ta.spend_limit as to_account_spend_limit,
    cta.id as to_account_currency_id,
    cta.name as to_account_currency_name,
    cta.symbol as to_account_currency_symbol,
    cta.currency as to_account_currency_code,
    cta.decimal_places as to_account_currency_decimal_places,
    cta.created_at as to_account_currency_created_at,
    v.id as vendor_id,
    v.name as vendor_name,
    v.created_at as vendor_created_at
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

/// Builds a complete SELECT query for transactions with the specified table/CTE name and WHERE clause
fn build_transaction_query(from_clause: &str, where_clause: &str, order_by: &str) -> String {
    let mut query = format!("SELECT {} FROM {} {}", TRANSACTION_SELECT_FIELDS, from_clause, TRANSACTION_JOINS);

    if !where_clause.is_empty() {
        query.push_str("WHERE ");
        query.push_str(where_clause);
    }

    if !order_by.is_empty() {
        query.push_str(" ORDER BY ");
        query.push_str(order_by);
    }

    query
}

#[async_trait::async_trait]
pub trait TransactionRepository {
    async fn create_transaction(&self, transaction: &TransactionRequest) -> Result<Transaction, AppError>;

    async fn get_transaction_by_id(&self, id: &Uuid) -> Result<Option<Transaction>, AppError>;
    async fn list_transactions(&self, pagination: Option<&PaginationParams>) -> Result<(Vec<Transaction>, i64), AppError>;
    async fn get_transactions_for_period(&self, period_id: &Uuid, pagination: Option<&PaginationParams>) -> Result<(Vec<Transaction>, i64), AppError>;
    async fn delete_transaction(&self, id: &Uuid) -> Result<(), AppError>;
    async fn update_transaction(&self, id: &Uuid, transaction: &TransactionRequest) -> Result<Transaction, AppError>;
}

#[async_trait::async_trait]
impl TransactionRepository for PostgresRepository {
    async fn create_transaction(&self, transaction: &TransactionRequest) -> Result<Transaction, AppError> {
        let to_account_id = if let Some(acc_id) = &transaction.to_account_id { Some(acc_id) } else { None };
        let vendor_id = if let Some(v_id) = &transaction.vendor_id { Some(v_id) } else { None };

        let select_query = build_transaction_query("inserted t", "", "");
        let query = format!(
            r#"
            WITH inserted AS (
                INSERT INTO transaction (
                    amount,
                    description,
                    occurred_at,
                    category_id,
                    from_account_id,
                    to_account_id,
                    vendor_id
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7)
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
            .bind(to_account_id)
            .bind(vendor_id)
            .fetch_one(&self.pool)
            .await?;

        Ok(Transaction::from(row))
    }

    async fn get_transaction_by_id(&self, id: &Uuid) -> Result<Option<Transaction>, AppError> {
        let query = build_transaction_query("transaction t", "t.id = $1", "");
        let row = sqlx::query_as::<_, TransactionRow>(&query).bind(id).fetch_optional(&self.pool).await?;

        Ok(row.map(Transaction::from))
    }

    async fn list_transactions(&self, pagination: Option<&PaginationParams>) -> Result<(Vec<Transaction>, i64), AppError> {
        // Get total count
        #[derive(sqlx::FromRow)]
        struct CountRow {
            total: i64,
        }

        let count_row = sqlx::query_as::<_, CountRow>("SELECT COUNT(*) as total FROM transaction")
            .fetch_one(&self.pool)
            .await?;
        let total = count_row.total;

        // Build query with optional pagination
        let mut query = build_transaction_query("transaction t", "", "occurred_at DESC, t.created_at DESC");

        // Add pagination if requested
        if let Some(params) = pagination
            && let (Some(limit), Some(offset)) = (params.effective_limit(), params.offset())
        {
            query.push_str(&format!(" LIMIT {} OFFSET {}", limit, offset));
        }

        let rows = sqlx::query_as::<_, TransactionRow>(&query).fetch_all(&self.pool).await?;

        let transactions: Vec<Transaction> = rows.into_iter().map(Transaction::from).collect();

        Ok((transactions, total))
    }

    async fn get_transactions_for_period(&self, period_id: &Uuid, pagination: Option<&PaginationParams>) -> Result<(Vec<Transaction>, i64), AppError> {
        // Get total count for this period
        #[derive(sqlx::FromRow)]
        struct CountRow {
            total: i64,
        }

        let count_row = sqlx::query_as::<_, CountRow>(
            r#"
            SELECT COUNT(*) as total
            FROM transaction t
            CROSS JOIN budget_period bp
            WHERE bp.id = $1
                AND t.occurred_at >= bp.start_date
                AND t.occurred_at <= bp.end_date
            "#,
        )
        .bind(period_id)
        .fetch_one(&self.pool)
        .await?;
        let total = count_row.total;

        // Build query with budget_period cross join and WHERE clause
        let mut query = format!(
            "SELECT {} FROM transaction t CROSS JOIN budget_period bp {} WHERE bp.id = $1 AND t.occurred_at >= bp.start_date AND t.occurred_at <= bp.end_date ORDER BY occurred_at DESC, t.created_at DESC",
            TRANSACTION_SELECT_FIELDS, TRANSACTION_JOINS
        );

        // Add pagination if requested
        if let Some(params) = pagination
            && let (Some(limit), Some(offset)) = (params.effective_limit(), params.offset())
        {
            query.push_str(&format!(" LIMIT {} OFFSET {}", limit, offset));
        }

        let rows = sqlx::query_as::<_, TransactionRow>(&query).bind(period_id).fetch_all(&self.pool).await?;

        let transactions: Vec<Transaction> = rows.into_iter().map(Transaction::from).collect();

        Ok((transactions, total))
    }

    async fn delete_transaction(&self, id: &Uuid) -> Result<(), AppError> {
        sqlx::query("DELETE FROM transaction WHERE id = $1").bind(id).execute(&self.pool).await?;

        Ok(())
    }

    async fn update_transaction(&self, id: &Uuid, transaction: &TransactionRequest) -> Result<Transaction, AppError> {
        let select_query = build_transaction_query("updated t", "", "");
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
                WHERE id = $8
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
            .fetch_one(&self.pool)
            .await?;

        Ok(Transaction::from(row))
    }
}
