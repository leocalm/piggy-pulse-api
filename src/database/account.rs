use crate::database::currency::get_currency_by_code;
use crate::error::app_error::AppError;
use crate::models::account::{Account, AccountRequest, AccountType, Currency};
use deadpool_postgres::Client;
use tokio_postgres::Row;
use uuid::Uuid;

pub async fn create_account(
    client: &Client,
    request: &AccountRequest,
) -> Result<Option<Account>, AppError> {
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
            created_at,
            updated_at,
            currency_id
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

        Ok(map_row_to_account(rows.first()))
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
            a.updated_at,
            c.id as currency_id,
            c.name as currency_name,
            c.symbol as currency_symbol,
            c.currency as currency_code,
            c.decimal_places as currency_decimal_places
        FROM account a
        JOIN currency c ON c.id = a.currency_id
        WHERE a.id = $1
        "#,
            &[id],
        )
        .await?;

    Ok(map_row_to_account_with_currency(rows.first()))
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
            a.updated_at,
            c.id as currency_id,
            c.name as currency_name,
            c.symbol as currency_symbol,
            c.currency as currency_code,
            c.decimal_places as currency_decimal_places
        FROM account a
        JOIN currency c ON c.id = a.currency_id
        ORDER BY a.created_at DESC
        "#,
            &[],
        )
        .await?;

    Ok(rows
        .into_iter()
        .filter_map(|row| map_row_to_account_with_currency(Some(&row)))
        .collect())
}

pub async fn update_account(
    client: &Client,
    id: &Uuid,
    request: &AccountRequest,
) -> Result<Option<Account>, AppError> {
    if let Some(currency) = get_currency_by_code(client, &request.currency).await? {
        let rows = client
            .query(
                r#"
        UPDATE account
        SET
            name = $1,
            color = $2,
            icon = $3,
            account_type = $4::text::account_type,
            currency_id = $5,
            balance = $6
        WHERE id = $7
        RETURNING
            id,
            name,
            color,
            icon,
            account_type::text as account_type,
            balance,
            created_at,
            updated_at,
            currency_id
        "#,
                &[
                    &request.name,
                    &request.color,
                    &request.icon,
                    &request.account_type_to_db(),
                    &currency.id,
                    &request.balance,
                    id,
                ],
            )
            .await?;

        Ok(map_row_to_account(rows.first()))
    } else {
        Err(AppError::CurrencyDoesNotExist(request.currency.clone()))
    }
}

fn map_row_to_account(row: Option<&Row>) -> Option<Account> {
    row.map(|r| Account {
        id: r.get("id"),
        name: r.get("name"),
        color: r.get("color"),
        icon: r.get("icon"),
        account_type: account_type_from_db(r.get::<_, &str>("account_type")),
        currency: Currency {
            id: r.get("currency_id"),
            name: String::new(),
            symbol: String::new(),
            currency: String::new(),
            decimal_places: 2,
        },
        balance: r.get("balance"),
        created_at: r.get("created_at"),
        updated_at: r.get("updated_at"),
    })
}

fn map_row_to_account_with_currency(row: Option<&Row>) -> Option<Account> {
    row.map(|r| Account {
        id: r.get("id"),
        name: r.get("name"),
        color: r.get("color"),
        icon: r.get("icon"),
        account_type: account_type_from_db(r.get::<_, &str>("account_type")),
        currency: Currency {
            id: r.get("currency_id"),
            name: r.get("currency_name"),
            symbol: r.get("currency_symbol"),
            currency: r.get("currency_code"),
            decimal_places: r.get::<_, i16>("currency_decimal_places") as usize,
        },
        balance: r.get("balance"),
        created_at: r.get("created_at"),
        updated_at: r.get("updated_at"),
    })
}

fn account_type_from_db<T: AsRef<str>>(value: T) -> AccountType {
    match value.as_ref() {
        "Checking" => AccountType::Checking,
        "Savings" => AccountType::Savings,
        "CreditCard" => AccountType::CreditCard,
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
        }
    }
}
