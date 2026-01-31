use rocket::http::Status;
use rocket::response::Responder;
use rocket::{Request, Response};
use std::io::Cursor;
use thiserror::Error;
use tracing::error;
use validator::ValidationErrors;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Internal server error")]
    Db {
        message: String,
        #[source]
        source: Option<tokio_postgres::error::Error>,
    },
    #[error("Internal server error")]
    Pool {
        message: String,
        #[source]
        source: deadpool_postgres::PoolError,
    },
    #[error("User not found")]
    UserNotFound,
    #[error("Unauthorized")]
    Unauthorized,
    #[error("Invalid credentials")]
    InvalidCredentials,
    #[error("Internal server error")]
    PasswordHash { message: String },
    #[error("User {0} already exists")]
    UserAlreadyExists(String),
    #[error("Bad request: {0}")]
    BadRequest(String),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Currency not found: {0}")]
    CurrencyDoesNotExist(String),
    #[error("Internal server error")]
    UuidError {
        message: String,
        #[source]
        source: uuid::Error,
    },
    #[error("Validation error: {0}")]
    ValidationError(#[from] ValidationErrors),
    #[error("Internal server error")]
    ConfigurationError {
        message: String,
        #[source]
        source: figment::Error,
    },
}

impl AppError {
    pub fn db(message: impl Into<String>, source: tokio_postgres::error::Error) -> Self {
        Self::Db {
            message: message.into(),
            source: Some(source),
        }
    }

    pub fn db_message(message: impl Into<String>) -> Self {
        Self::Db {
            message: message.into(),
            source: None,
        }
    }

    pub fn pool(message: impl Into<String>, source: deadpool_postgres::PoolError) -> Self {
        Self::Pool {
            message: message.into(),
            source,
        }
    }

    pub fn uuid(message: impl Into<String>, source: uuid::Error) -> Self {
        Self::UuidError {
            message: message.into(),
            source,
        }
    }

    pub fn password_hash(message: impl Into<String>, source: password_hash::Error) -> Self {
        Self::PasswordHash {
            message: format!("{}: {}", message.into(), source),
        }
    }
}

impl From<tokio_postgres::error::Error> for AppError {
    fn from(e: tokio_postgres::error::Error) -> Self {
        AppError::db("Database operation failed", e)
    }
}

impl From<password_hash::Error> for AppError {
    fn from(e: password_hash::Error) -> Self {
        AppError::password_hash("Password hashing failed", e)
    }
}

impl From<uuid::Error> for AppError {
    fn from(e: uuid::Error) -> Self {
        AppError::uuid("Invalid UUID", e)
    }
}

impl From<&AppError> for Status {
    fn from(e: &AppError) -> Self {
        match e {
            AppError::UserNotFound => Status::NotFound,
            AppError::InvalidCredentials => Status::Forbidden,
            AppError::PasswordHash { .. } => Status::InternalServerError,
            AppError::Db { .. } => Status::InternalServerError,
            AppError::Pool { .. } => Status::InternalServerError,
            AppError::Unauthorized => Status::Unauthorized,
            AppError::UserAlreadyExists(_) => Status::Conflict,
            AppError::BadRequest(_) => Status::BadRequest,
            AppError::NotFound(_) => Status::NotFound,
            AppError::CurrencyDoesNotExist(_) => Status::BadRequest,
            AppError::UuidError { .. } => Status::BadRequest,
            AppError::ValidationError(_) => Status::BadRequest,
            AppError::ConfigurationError { .. } => Status::InternalServerError,
        }
    }
}

impl<'r> Responder<'r, 'static> for AppError {
    fn respond_to(self, _req: &Request<'_>) -> rocket::response::Result<'static> {
        error!(error = ?self, "request failed");

        let status = Status::from(&self);
        let body = self.to_string();

        Response::build().status(status).sized_body(body.len(), Cursor::new(body)).ok()
    }
}

impl From<figment::Error> for AppError {
    fn from(e: figment::Error) -> Self {
        AppError::ConfigurationError {
            message: "Failed to read configuration".to_string(),
            source: e,
        }
    }
}
