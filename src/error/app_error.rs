use rocket::http::Status;
use rocket::response::Responder;
use rocket::{Request, Response};
use serde::Deserialize;
use std::io::Cursor;
use tracing::error;
use validator::ValidationErrors;

#[derive(Debug, Deserialize)]
pub enum AppError {
    Db(String),
    UserNotFound,
    Unauthorized,
    InvalidCredentials,
    PasswordHash(String),
    UserAlreadyExists(String),
    BadRequest(String),
    NotFound(String),
    CurrencyDoesNotExist(String),
    UuidError(String),
    ValidationError(String),
    ConfigurationError(),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::UserNotFound => write!(f, "User not found"),
            Self::InvalidCredentials => write!(f, "Invalid credentials"),
            Self::Unauthorized => write!(f, "Unauthorized"),
            Self::Db(_) => write!(f, "Internal server error"),
            Self::PasswordHash(_) => write!(f, "Internal server error"),
            Self::UserAlreadyExists(s) => write!(f, "User {} already exists", s),
            Self::BadRequest(s) => write!(f, "Bad request: {}", s),
            Self::NotFound(s) => write!(f, "Not found: {}", s),
            Self::CurrencyDoesNotExist(s) => write!(f, "Currency not found: {}", s),
            Self::UuidError(_) => write!(f, "Internal server error"),
            Self::ValidationError(e) => write!(f, "Validation error: {}", e),
            Self::ConfigurationError() => write!(f, "Internal server error"),
        }
    }
}

impl std::error::Error for AppError {}

impl From<tokio_postgres::error::Error> for AppError {
    fn from(e: tokio_postgres::error::Error) -> Self {
        log::error!("{}", e);
        AppError::Db(e.to_string())
    }
}

impl From<password_hash::Error> for AppError {
    fn from(e: password_hash::Error) -> Self {
        log::error!("{}", e);
        AppError::PasswordHash(e.to_string())
    }
}

impl From<uuid::Error> for AppError {
    fn from(e: uuid::Error) -> Self {
        log::error!("{}", e);
        AppError::UuidError(e.to_string())
    }
}

impl From<&AppError> for Status {
    fn from(e: &AppError) -> Self {
        match e {
            AppError::UserNotFound => Status::NotFound,
            AppError::InvalidCredentials => Status::Forbidden,
            AppError::PasswordHash(_) => Status::InternalServerError,
            AppError::Db(_) => Status::InternalServerError,
            AppError::Unauthorized => Status::Unauthorized,
            AppError::UserAlreadyExists(_) => Status::Conflict,
            AppError::BadRequest(_) => Status::BadRequest,
            AppError::NotFound(_) => Status::NotFound,
            AppError::CurrencyDoesNotExist(_) => Status::BadRequest,
            AppError::UuidError(_) => Status::BadRequest,
            AppError::ValidationError(_) => Status::BadRequest,
            AppError::ConfigurationError() => Status::InternalServerError,
        }
    }
}

impl<'r> Responder<'r, 'static> for AppError {
    fn respond_to(self, _req: &Request<'_>) -> rocket::response::Result<'static> {
        error!("{}", self);

        let status = Status::from(&self);
        let body = self.to_string();

        Response::build().status(status).sized_body(body.len(), Cursor::new(body)).ok()
    }
}

impl From<ValidationErrors> for AppError {
    fn from(e: ValidationErrors) -> Self {
        AppError::ValidationError(e.to_string())
    }
}

impl From<figment::Error> for AppError {
    fn from(e: figment::Error) -> Self {
        log::error!("Error reading configuration: {}", e);
        AppError::ConfigurationError()
    }
}
