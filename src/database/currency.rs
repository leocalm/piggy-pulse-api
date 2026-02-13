use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::models::currency::Currency;
use uuid::Uuid;

impl PostgresRepository {
    pub async fn get_currency_by_code(&self, currency_code: &str) -> Result<Option<Currency>, AppError> {
        Ok(sqlx::query_as::<_, Currency>(
            r#"
              SELECT id, name, symbol, currency, decimal_places, symbol_position
              FROM currency
              WHERE currency = $1
              LIMIT 1
            "#,
        )
        .bind(currency_code)
        .fetch_optional(&self.pool)
        .await?)
    }

    pub async fn get_currency_by_id(&self, id: &Uuid) -> Result<Option<Currency>, AppError> {
        Ok(sqlx::query_as::<_, Currency>(
            r#"
              SELECT id, name, symbol, currency, decimal_places, symbol_position
              FROM currency
              WHERE id = $1
              LIMIT 1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?)
    }

    pub async fn get_currencies_by_name(&self, name: &str) -> Result<Vec<Currency>, AppError> {
        let pattern = format!("%{}%", name);

        Ok(sqlx::query_as::<_, Currency>(
            r#"
        SELECT id, name, symbol, currency, decimal_places, symbol_position
        FROM currency
        WHERE lower(name) LIKE lower($1)
        ORDER BY name ASC
        "#,
        )
        .bind(pattern)
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn get_all_currencies(&self) -> Result<Vec<Currency>, AppError> {
        Ok(sqlx::query_as::<_, Currency>(
            r#"
        SELECT id, name, symbol, currency, decimal_places, symbol_position
        FROM currency
        ORDER BY name ASC
        "#,
        )
        .fetch_all(&self.pool)
        .await?)
    }
}
