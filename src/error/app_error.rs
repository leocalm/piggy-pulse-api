use rocket::http::Status;
use rocket::response::Responder;
use rocket::{Request, Response};
use std::io::Cursor;
use tracing::error;

#[derive(Debug)]
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
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::UserNotFound => write!(f, "User not found"),
            Self::InvalidCredentials => write!(f, "Invalid credentials"),
            Self::Unauthorized => write!(f, "Unauthorized"),
            Self::Db(s) => write!(f, "Database error: {}", s),
            Self::PasswordHash(s) => write!(f, "Password hash error: {}", s),
            Self::UserAlreadyExists(s) => write!(f, "User {} already exists", s),
            Self::BadRequest(s) => write!(f, "Bad request: {}", s),
            Self::NotFound(s) => write!(f, "Not found: {}", s),
            Self::CurrencyDoesNotExist(s) => write!(f, "Currency not found: {}", s),
            Self::UuidError(s) => write!(f, "Uuid error: {}", s),
        }
    }
}

impl std::error::Error for AppError {}

impl From<tokio_postgres::error::Error> for AppError {
    fn from(e: tokio_postgres::error::Error) -> Self {
        AppError::Db(e.to_string())
    }
}

impl From<password_hash::Error> for AppError {
    fn from(e: password_hash::Error) -> Self {
        AppError::PasswordHash(e.to_string())
    }
}

impl From<uuid::Error> for AppError {
    fn from(e: uuid::Error) -> Self {
        AppError::UserAlreadyExists(e.to_string())
    }
}

impl From<AppError> for Status {
    fn from(e: AppError) -> Self {
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
        }
    }
}

impl<'r> Responder<'r, 'static> for AppError {
    fn respond_to(self, _req: &Request<'_>) -> rocket::response::Result<'static> {
        error!("{}", self);

        let status = Status::from(self);
        let body = status.to_string();

        Response::build()
            .status(status)
            .sized_body(body.len(), Cursor::new(body))
            .ok()
    }
}
