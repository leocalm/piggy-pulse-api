use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::models::currency::{Currency, CurrencyRequest};
use tokio_postgres::Row;
use uuid::Uuid;

#[async_trait::async_trait]
pub trait CurrencyRepository {
    async fn get_currency_by_code(&self, currency_code: &str) -> Result<Option<Currency>, AppError>;
    async fn get_currencies(&self, name: &str) -> Result<Vec<Currency>, AppError>;
    async fn create_currency(&self, currency: &CurrencyRequest) -> Result<Currency, AppError>;
    async fn delete_currency(&self, currency_id: &Uuid) -> Result<(), AppError>;
    async fn update_currency(&self, id: &Uuid, request: &CurrencyRequest) -> Result<Currency, AppError>;
}

#[async_trait::async_trait]
impl<'a> CurrencyRepository for PostgresRepository<'a> {
    async fn get_currency_by_code(&self, currency_code: &str) -> Result<Option<Currency>, AppError> {
        let rows = self
            .client
            .query(
                r#"
        SELECT id, name, symbol, currency, decimal_places, created_at
        FROM currency
        WHERE currency = $1
        "#,
                &[&currency_code],
            )
            .await
            .map_err(|e| AppError::db("Failed to fetch currency by code", e))?;

        if let Some(row) = rows.first() {
            Ok(Some(map_row_to_currency(row)))
        } else {
            Ok(None)
        }
    }

    async fn get_currencies(&self, name: &str) -> Result<Vec<Currency>, AppError> {
        let rows = self
            .client
            .query(
                r#"
        SELECT id, name, symbol, currency, decimal_places, created_at
        FROM currency
        WHERE lower(name) LIKE lower($1)
        "#,
                &[&format!("%{}%", name)],
            )
            .await
            .map_err(|e| AppError::db("Failed to search currencies", e))?;

        Ok(rows.iter().map(map_row_to_currency).collect())
    }

    async fn create_currency(&self, currency: &CurrencyRequest) -> Result<Currency, AppError> {
        let rows = self
            .client
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
            created_at
        "#,
                &[&currency.name, &currency.symbol, &currency.currency, &currency.decimal_places],
            )
            .await
            .map_err(|e| AppError::db("Failed to create currency", e))?;

        if let Some(row) = rows.first() {
            Ok(map_row_to_currency(row))
        } else {
            Err(AppError::db_message("Error mapping created currency"))
        }
    }

    async fn delete_currency(&self, currency_id: &Uuid) -> Result<(), AppError> {
        self.client
            .execute(
                r#"
        DELETE FROM currency
        WHERE id = $1
        "#,
                &[&currency_id],
            )
            .await
            .map_err(|e| AppError::db("Failed to delete currency", e))?;

        Ok(())
    }

    async fn update_currency(&self, id: &Uuid, request: &CurrencyRequest) -> Result<Currency, AppError> {
        let rows = self
            .client
            .query(
                r#"
            UPDATE currency
            SET name = $1, symbol = $2, currency = $3, decimal_places = $4
            WHERE id = $5
            RETURNING id, name, symbol, currency, decimal_places, created_at
            "#,
                &[&request.name, &request.symbol, &request.currency, &request.decimal_places, &id],
            )
            .await
            .map_err(|e| AppError::db("Failed to update currency", e))?;

        if let Some(row) = rows.first() {
            Ok(map_row_to_currency(row))
        } else {
            Err(AppError::NotFound("Currency not found".to_string()))
        }
    }
}

fn map_row_to_currency(row: &Row) -> Currency {
    Currency {
        id: row.get("id"),
        name: row.get("name"),
        symbol: row.get("symbol"),
        currency: row.get("currency"),
        decimal_places: row.get::<_, i32>("decimal_places"),
        created_at: row.get("created_at"),
    }
}
