use crate::database::postgres_repository::PostgresRepository;
use crate::dto::misc::{CurrencyListResponse, CurrencyResponse, SymbolPosition as DtoSymbolPosition};
use crate::error::app_error::AppError;
use crate::models::currency::{Currency, SymbolPosition as ModelSymbolPosition};

pub struct CurrencyService<'a> {
    repository: &'a PostgresRepository,
}

impl<'a> CurrencyService<'a> {
    pub fn new(repository: &'a PostgresRepository) -> Self {
        CurrencyService { repository }
    }

    pub async fn list_currencies(&self) -> Result<CurrencyListResponse, AppError> {
        let currencies = self.repository.get_all_currencies().await?;
        Ok(currencies.iter().map(to_dto).collect())
    }

    pub async fn get_currency_by_code(&self, code: &str) -> Result<CurrencyResponse, AppError> {
        let currency = self
            .repository
            .get_currency_by_code(code)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Currency not found: {}", code)))?;
        Ok(to_dto(&currency))
    }
}

fn to_dto(currency: &Currency) -> CurrencyResponse {
    CurrencyResponse {
        id: currency.id,
        name: currency.name.clone(),
        symbol: currency.symbol.clone(),
        code: currency.currency.clone(),
        decimal_places: currency.decimal_places,
        symbol_position: match currency.symbol_position {
            ModelSymbolPosition::Before => DtoSymbolPosition::Before,
            ModelSymbolPosition::After => DtoSymbolPosition::After,
        },
    }
}
