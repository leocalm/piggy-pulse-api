use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::models::currency::{Currency, CurrencyRequest};
use uuid::Uuid;

impl PostgresRepository {
    pub async fn get_currency_by_code(&self, currency_code: &str, user_id: &Uuid) -> Result<Option<Currency>, AppError> {
        Ok(sqlx::query_as::<_, Currency>(
            r#"
              SELECT id, name, symbol, currency, decimal_places, created_at
              FROM currency
              WHERE currency = $1
                AND (user_id IS NULL OR user_id = $2)
              ORDER BY user_id NULLS LAST
              LIMIT 1
            "#,
        )
        .bind(currency_code)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?)
    }

    pub async fn get_currencies(&self, name: &str, user_id: &Uuid) -> Result<Vec<Currency>, AppError> {
        let pattern = format!("%{}%", name);

        Ok(sqlx::query_as::<_, Currency>(
            r#"
        SELECT id, name, symbol, currency, decimal_places, created_at
        FROM currency
        WHERE lower(name) LIKE lower($1)
          AND (user_id IS NULL OR user_id = $2)
        ORDER BY user_id NULLS LAST, name ASC
        "#,
        )
        .bind(pattern)
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn create_currency(&self, currency: &CurrencyRequest, user_id: &Uuid) -> Result<Currency, AppError> {
        Ok(sqlx::query_as::<_, Currency>(
            r#"
        INSERT INTO currency (user_id, name, symbol, currency, decimal_places)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING
            id,
            name,
            symbol,
            currency,
            decimal_places,
            created_at
        "#,
        )
        .bind(user_id)
        .bind(&currency.name)
        .bind(&currency.symbol)
        .bind(&currency.currency)
        .bind(currency.decimal_places)
        .fetch_one(&self.pool)
        .await?)
    }

    pub async fn delete_currency(&self, currency_id: &Uuid, user_id: &Uuid) -> Result<(), AppError> {
        sqlx::query(
            r#"
        DELETE FROM currency
        WHERE id = $1
          AND user_id = $2
        "#,
        )
        .bind(currency_id)
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn update_currency(&self, id: &Uuid, request: &CurrencyRequest, user_id: &Uuid) -> Result<Currency, AppError> {
        Ok(sqlx::query_as::<_, Currency>(
            r#"
            UPDATE currency
            SET name = $1, symbol = $2, currency = $3, decimal_places = $4
            WHERE id = $5
              AND user_id = $6
            RETURNING id, name, symbol, currency, decimal_places, created_at
            "#,
        )
        .bind(&request.name)
        .bind(&request.symbol)
        .bind(&request.currency)
        .bind(request.decimal_places)
        .bind(id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?)
    }
}
