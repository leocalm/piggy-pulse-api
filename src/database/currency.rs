use crate::error::app_error::AppError;
use crate::models::currency::{Currency, CurrencyRequest};
use deadpool_postgres::Client;
use tokio_postgres::Row;
use uuid::Uuid;

pub async fn get_currency_by_code(
    client: &Client,
    currency_code: &str,
) -> Result<Option<Currency>, AppError> {
    let rows = client
        .query(
            r#"
        SELECT id, name, symbol, currency, decimal_places, created_at, deleted, deleted_at
        FROM currency
        WHERE currency = $1
        AND deleted = false
        "#,
            &[&currency_code],
        )
        .await?;

    if let Some(row) = rows.first() {
        Ok(Some(map_row_to_currency(row)))
    } else {
        Ok(None)
    }
}

pub async fn get_currencies(client: &Client, name: &str) -> Result<Vec<Currency>, AppError> {
    let rows = client
        .query(
            r#"
        SELECT id, name, symbol, currency, decimal_places, created_at, deleted, deleted_at
        FROM currency
        WHERE lower(name) LIKE lower($1)
        AND deleted = false
        "#,
            &[&format!("%{}%", name)],
        )
        .await?;

    Ok(rows.iter().map(map_row_to_currency).collect())
}

pub async fn create_currency(
    client: &Client,
    currency: &CurrencyRequest,
) -> Result<Currency, AppError> {
    let rows = client
        .query(
            r#"
        INSERT INTO currency (name, symbol, currency, decimal_places)
        VALUES ($1, $2, $3, $4)
        RETURNING
            id,
            name,
            symbol,
            currency,
            decimal_places,
            created_at,
            deleted,
            deleted_at
        "#,
            &[
                &currency.name,
                &currency.symbol,
                &currency.currency,
                &currency.decimal_places,
            ],
        )
        .await?;

    if let Some(row) = rows.first() {
        Ok(map_row_to_currency(row))
    } else {
        Err(AppError::Db("Error mapping created currency".into()))
    }
}

pub async fn delete_currency(client: &Client, currency_id: &Uuid) -> Result<(), AppError> {
    client
        .execute(
            r#"
        UPDATE currency
        SET deleted = true,
            deleted_at = now()
        WHERE id = $1
        "#,
            &[&currency_id],
        )
        .await?;

    Ok(())
}

fn map_row_to_currency(row: &Row) -> Currency {
    Currency {
        id: row.get("id"),
        name: row.get("name"),
        symbol: row.get("symbol"),
        currency: row.get("currency"),
        decimal_places: row.get::<_, i32>("decimal_places"),
        created_at: row.get("created_at"),
        deleted: row.get("deleted"),
        deleted_at: row.get("deleted_at"),
    }
}
