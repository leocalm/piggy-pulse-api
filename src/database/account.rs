use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::models::account::{Account, AccountRequest, AccountType};
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

impl PostgresRepository {
    pub async fn create_account(&self, request: &AccountRequest, user_id: &Uuid) -> Result<Account, AppError> {
        let currency = self
            .get_currency_by_code(&request.currency)
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
        .bind(&request.name)
        .bind(user_id)
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

    pub async fn list_accounts(&self, params: &CursorParams, user_id: &Uuid) -> Result<Vec<Account>, AppError> {
        let rows = if let Some(cursor) = params.cursor {
            sqlx::query_as::<_, AccountRow>(
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
                WHERE (a.created_at, a.id) < (
                    SELECT created_at, id FROM account WHERE id = $1
                ) AND a.user_id = $2
                ORDER BY a.created_at DESC, a.id DESC
                LIMIT $3
                "#,
            )
            .bind(cursor)
            .bind(user_id)
            .bind(params.fetch_limit())
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, AccountRow>(
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
                WHERE a.user_id = $1
                ORDER BY a.created_at DESC, a.id DESC
                LIMIT $2
                "#,
            )
            .bind(user_id)
            .bind(params.fetch_limit())
            .fetch_all(&self.pool)
            .await?
        };

        Ok(rows.into_iter().map(Account::from).collect())
    }

    pub async fn delete_account(&self, id: &Uuid, user_id: &Uuid) -> Result<(), AppError> {
        sqlx::query("DELETE FROM account WHERE id = $1 AND user_id = $2")
            .bind(id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn update_account(&self, id: &Uuid, request: &AccountRequest, user_id: &Uuid) -> Result<Account, AppError> {
        let currency = self
            .get_currency_by_code(&request.currency)
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
        .await?;

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
