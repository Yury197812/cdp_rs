// database/sqlite/error.rs - Database errors

use std::fmt;

#[derive(Debug)]
pub enum DbError {
    ConnectionError(String),
    QueryError(String),
    NotFound(String),
    ValidationError(String),
}

impl fmt::Display for DbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DbError::ConnectionError(msg) => write!(f, "Connection error: {}", msg),
            DbError::QueryError(msg) => write!(f, "Query error: {}", msg),
            DbError::NotFound(msg) => write!(f, "Not found: {}", msg),
            DbError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
        }
    }
}

impl std::error::Error for DbError {}

impl From<String> for DbError {
    fn from(err: String) -> Self {
        DbError::QueryError(err)
    }
}
