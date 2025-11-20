use crate::error::app_error::AppError;
use crate::models::account::Currency;
use deadpool_postgres::Client;

pub async fn get_currency_by_code(
    client: &Client,
    currency_code: &str,
) -> Result<Option<Currency>, AppError> {
    let rows = client
        .query(
            r#"
        SELECT id, name, symbol, currency, decimal_places
        FROM currency
        WHERE currency = $1
        "#,
            &[&currency_code],
        )
        .await?;

    if let Some(row) = rows.first() {
        Ok(Some(Currency {
            id: row.get("id"),
            name: row.get("name"),
            symbol: row.get("symbol"),
            currency: row.get("currency"),
            decimal_places: row.get::<_, i16>("decimal_places") as usize,
        }))
    } else {
        Ok(None)
    }
}
