// database/sqlite/mod.rs - SQLite submodule
pub mod connection;
pub mod query;
pub mod error;

pub use connection::Database;
pub use query::QueryResult;
pub use error::DbError;
