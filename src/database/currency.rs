use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::models::currency::{Currency, CurrencyRequest};
use uuid::Uuid;

impl PostgresRepository {
    pub async fn get_currency_by_code(&self, currency_code: &str) -> Result<Option<Currency>, AppError> {
        Ok(sqlx::query_as::<_, Currency>(
            r#"
              SELECT id, name, symbol, currency, decimal_places, created_at
              FROM currency
              WHERE currency = $1
            "#,
        )
        .bind(currency_code)
        .fetch_optional(&self.pool)
        .await?)
    }

    pub async fn get_currencies(&self, name: &str) -> Result<Vec<Currency>, AppError> {
        let pattern = format!("%{}%", name);

        Ok(sqlx::query_as::<_, Currency>(
            r#"
        SELECT id, name, symbol, currency, decimal_places, created_at
        FROM currency
        WHERE lower(name) LIKE lower($1)
        "#,
        )
        .bind(pattern)
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn create_currency(&self, currency: &CurrencyRequest) -> Result<Currency, AppError> {
        Ok(sqlx::query_as::<_, Currency>(
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
        )
        .bind(&currency.name)
        .bind(&currency.symbol)
        .bind(&currency.currency)
        .bind(currency.decimal_places)
        .fetch_one(&self.pool)
        .await?)
    }

    pub async fn delete_currency(&self, currency_id: &Uuid) -> Result<(), AppError> {
        sqlx::query(
            r#"
        DELETE FROM currency
        WHERE id = $1
        "#,
        )
        .bind(currency_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn update_currency(&self, id: &Uuid, request: &CurrencyRequest) -> Result<Currency, AppError> {
        Ok(sqlx::query_as::<_, Currency>(
            r#"
            UPDATE currency
            SET name = $1, symbol = $2, currency = $3, decimal_places = $4
            WHERE id = $5
            RETURNING id, name, symbol, currency, decimal_places, created_at
            "#,
        )
        .bind(&request.name)
        .bind(&request.symbol)
        .bind(&request.currency)
        .bind(request.decimal_places)
        .bind(id)
        .fetch_one(&self.pool)
        .await?)
    }
}
