use crate::database::currency::CurrencyRepository;
use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::models::account::{Account, AccountRequest, AccountType};
use crate::models::currency::Currency;
use crate::models::pagination::PaginationParams;
use chrono::{DateTime, Utc};
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

#[async_trait::async_trait]
pub trait AccountRepository {
    async fn create_account(&self, request: &AccountRequest) -> Result<Account, AppError>;
    async fn get_account_by_id(&self, id: &Uuid) -> Result<Option<Account>, AppError>;
    async fn list_accounts(&self, pagination: Option<&PaginationParams>) -> Result<(Vec<Account>, i64), AppError>;
    async fn delete_account(&self, id: &Uuid) -> Result<(), AppError>;
    async fn update_account(&self, id: &Uuid, request: &AccountRequest) -> Result<Account, AppError>;
}

#[async_trait::async_trait]
impl AccountRepository for PostgresRepository {
    async fn create_account(&self, request: &AccountRequest) -> Result<Account, AppError> {
        let currency = self
            .get_currency_by_code(&request.currency)
            .await?
            .ok_or_else(|| AppError::CurrencyDoesNotExist(request.currency.clone()))?;

        let account_type_str = request.account_type_to_db();

        #[derive(sqlx::FromRow)]
        struct CreateAccountRow {
            id: Uuid,
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
            INSERT INTO account (name, color, icon, account_type, currency_id, balance, spend_limit)
            VALUES ($1, $2, $3, $4::text::account_type, $5, $6, $7)
            RETURNING
                id,
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
        .bind(request.spend_limit)
        .fetch_one(&self.pool)
        .await?;

        Ok(Account {
            id: row.id,
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

    async fn get_account_by_id(&self, id: &Uuid) -> Result<Option<Account>, AppError> {
        let row = sqlx::query_as::<_, AccountRow>(
            r#"
            SELECT
                a.id,
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
            WHERE a.id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Account::from))
    }

    async fn list_accounts(&self, pagination: Option<&PaginationParams>) -> Result<(Vec<Account>, i64), AppError> {
        // Get total count
        #[derive(sqlx::FromRow)]
        struct CountRow {
            total: i64,
        }

        let count_row = sqlx::query_as::<_, CountRow>("SELECT COUNT(*) as total FROM account")
            .fetch_one(&self.pool)
            .await?;
        let total = count_row.total;

        // Build query with optional pagination
        let base_query = r#"
            SELECT
                a.id,
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
            ORDER BY a.created_at DESC
        "#;

        let rows = if let Some(params) = pagination
            && let (Some(limit), Some(offset)) = (params.effective_limit(), params.offset())
        {
            sqlx::query_as::<_, AccountRow>(&format!("{} LIMIT $1 OFFSET $2", base_query))
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await?
        } else {
            sqlx::query_as::<_, AccountRow>(base_query).fetch_all(&self.pool).await?
        };

        let accounts: Vec<Account> = rows.into_iter().map(Account::from).collect();

        Ok((accounts, total))
    }

    async fn delete_account(&self, id: &Uuid) -> Result<(), AppError> {
        sqlx::query("DELETE FROM account WHERE id = $1").bind(id).execute(&self.pool).await?;

        Ok(())
    }

    async fn update_account(&self, id: &Uuid, request: &AccountRequest) -> Result<Account, AppError> {
        let currency = self
            .get_currency_by_code(&request.currency)
            .await?
            .ok_or_else(|| AppError::CurrencyDoesNotExist(request.currency.clone()))?;

        let account_type_str = request.account_type_to_db();

        #[derive(sqlx::FromRow)]
        struct UpdateAccountRow {
            id: Uuid,
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
            WHERE id = $7
            RETURNING
                id,
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
        .fetch_one(&self.pool)
        .await?;

        Ok(Account {
            id: row.id,
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
