use crate::database::currency::CurrencyRepository;
use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::models::account::{Account, AccountRequest, AccountType};
use crate::models::currency::Currency;
use tokio_postgres::Row;
use uuid::Uuid;

#[async_trait::async_trait]
pub trait AccountRepository {
    async fn create_account(&self, request: &AccountRequest) -> Result<Account, AppError>;
    async fn get_account_by_id(&self, id: &Uuid) -> Result<Option<Account>, AppError>;
    async fn list_accounts(&self) -> Result<Vec<Account>, AppError>;
    async fn delete_account(&self, id: &Uuid) -> Result<(), AppError>;
    async fn update_account(&self, id: &Uuid, request: &AccountRequest) -> Result<Account, AppError>;
}

#[async_trait::async_trait]
impl<'a> AccountRepository for PostgresRepository<'a> {
    async fn create_account(&self, request: &AccountRequest) -> Result<Account, AppError> {
        if let Some(currency) = self.get_currency_by_code(&request.currency).await? {
            let rows = self
                .client
                .query(
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
                    &[
                        &request.name,
                        &request.color,
                        &request.icon,
                        &request.account_type_to_db(),
                        &currency.id,
                        &request.balance,
                        &request.spend_limit,
                    ],
                )
                .await?;
            if let Some(row) = rows.first() {
                Ok(map_row_to_account(row, Some(currency)))
            } else {
                Err(AppError::Db("Error mapping created account".to_string()))
            }
        } else {
            Err(AppError::CurrencyDoesNotExist(request.currency.clone()))
        }
    }

    async fn get_account_by_id(&self, id: &Uuid) -> Result<Option<Account>, AppError> {
        let rows = self
            .client
            .query(
                r#"
        SELECT
            a.id,
            a.name,
            a.color,
            a.icon,
            a.account_type::text as account_type,
            a.balance,
            a.created_at,
            a.spend_limit as spend_limit,
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
                &[id],
            )
            .await?;

        if let Some(row) = rows.first() {
            Ok(Some(map_row_to_account(row, None)))
        } else {
            Ok(None)
        }
    }

    async fn list_accounts(&self) -> Result<Vec<Account>, AppError> {
        let rows = self
            .client
            .query(
                r#"
            SELECT
                a.id,
                a.name,
                a.color,
                a.icon,
                a.account_type::text as account_type,
                a.balance,
                a.created_at,
                a.spend_limit as spend_limit,
                c.id as currency_id,
                c.name as currency_name,
                c.symbol as currency_symbol,
                c.currency as currency_code,
                c.decimal_places as currency_decimal_places,
                c.created_at as currency_created_at
            FROM account a
                     JOIN currency c ON c.id = a.currency_id
            ORDER BY a.created_at DESC
        "#,
                &[],
            )
            .await?;

        Ok(rows.into_iter().map(|row| map_row_to_account(&row, None)).collect())
    }

    async fn delete_account(&self, id: &Uuid) -> Result<(), AppError> {
        self.client
            .execute(
                r#"
        DELETE FROM account
        WHERE id = $1
        "#,
                &[&id],
            )
            .await?;

        Ok(())
    }

    async fn update_account(&self, id: &Uuid, request: &AccountRequest) -> Result<Account, AppError> {
        if let Some(currency) = self.get_currency_by_code(&request.currency).await? {
            let rows = self
                .client
                .query(
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
                    &[
                        &request.name,
                        &request.color,
                        &request.icon,
                        &request.account_type_to_db(),
                        &currency.id,
                        &request.balance,
                        &id,
                    ],
                )
                .await?;

            if let Some(row) = rows.first() {
                Ok(map_row_to_account(row, Some(currency)))
            } else {
                Err(AppError::NotFound("Account not found".to_string()))
            }
        } else {
            Err(AppError::CurrencyDoesNotExist(request.currency.clone()))
        }
    }
}

fn map_row_to_account(row: &Row, currency_opt: Option<Currency>) -> Account {
    let currency = if let Some(currency_request) = currency_opt {
        currency_request
    } else {
        Currency {
            id: row.get("currency_id"),
            name: row.get("currency_name"),
            symbol: row.get("currency_symbol"),
            currency: row.get("currency_code"),
            decimal_places: row.get("currency_decimal_places"),
            created_at: row.get("currency_created_at"),
        }
    };

    Account {
        id: row.get("id"),
        name: row.get("name"),
        color: row.get("color"),
        icon: row.get("icon"),
        account_type: account_type_from_db(row.get::<_, &str>("account_type")),
        currency,
        balance: row.get("balance"),
        created_at: row.get("created_at"),
        spend_limit: row.get("spend_limit"),
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
