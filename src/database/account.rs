use crate::database::currency::get_currency_by_code;
use crate::error::app_error::AppError;
use crate::models::account::{Account, AccountRequest, AccountType};
use crate::models::currency::Currency;
use deadpool_postgres::Client;
use tokio_postgres::Row;
use uuid::Uuid;

pub async fn create_account(
    client: &Client,
    request: &AccountRequest,
) -> Result<Account, AppError> {
    if let Some(currency) = get_currency_by_code(client, &request.currency).await? {
        let rows = client
            .query(
                r#"
        INSERT INTO account (name, color, icon, account_type, currency_id, balance)
        VALUES ($1, $2, $3, $4::text::account_type, $5, $6)
        RETURNING
            id,
            name,
            color,
            icon,
            account_type::text as account_type,
            balance,
            currency_id,
            created_at,
            deleted,
            deleted_at
        "#,
                &[
                    &request.name,
                    &request.color,
                    &request.icon,
                    &request.account_type_to_db(),
                    &currency.id,
                    &request.balance,
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

pub async fn get_account_by_id(client: &Client, id: &Uuid) -> Result<Option<Account>, AppError> {
    let rows = client
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
            a.deleted,
            a.deleted_at,
            c.id as currency_id,
            c.name as currency_name,
            c.symbol as currency_symbol,
            c.currency as currency_code,
            c.decimal_places as currency_decimal_places,
            c.created_at as currency_created_at,
            c.deleted as currency_deleted,
            c.deleted_at as currency_deleted_at
        FROM account a
        JOIN currency c ON c.id = a.currency_id
        WHERE a.id = $1
            AND a.deleted = false
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

pub async fn list_accounts(client: &Client) -> Result<Vec<Account>, AppError> {
    let rows = client
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
                a.deleted,
                a.deleted_at,
                c.id as currency_id,
                c.name as currency_name,
                c.symbol as currency_symbol,
                c.currency as currency_code,
                c.decimal_places as currency_decimal_places,
                c.created_at as currency_created_at,
                c.deleted as currency_deleted,
                c.deleted_at as currency_deleted_at
            FROM account a
                     JOIN currency c ON c.id = a.currency_id
            WHERE a.deleted = false
            ORDER BY a.created_at DESC
        "#,
            &[],
        )
        .await?;

    Ok(rows
        .into_iter()
        .map(|row| map_row_to_account(&row, None))
        .collect())
}

pub async fn delete_account(client: &Client, id: &Uuid) -> Result<(), AppError> {
    client
        .execute(
            r#"
        UPDATE account
        SET deleted = true,
            deleted_at = now()
        WHERE id = $1
        "#,
            &[&id],
        )
        .await?;

    Ok(())
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
            deleted: row.get("currency_deleted"),
            deleted_at: row.get("currency_deleted_at"),
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
        deleted: row.get("deleted"),
        deleted_at: row.get("deleted_at"),
    }
}

pub fn account_type_from_db<T: AsRef<str>>(value: T) -> AccountType {
    match value.as_ref() {
        "Checking" => AccountType::Checking,
        "Savings" => AccountType::Savings,
        "CreditCard" => AccountType::CreditCard,
        "Wallet" => AccountType::Wallet,
        other => panic!("Unknown account type: {}", other),
    }
}

// Helper method for AccountRequest to map to DB enum/text value
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
        }
    }
}
