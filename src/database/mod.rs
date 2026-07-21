// database/mod.rs - Database module
pub mod sqlite;
pub mod pool;
pub mod models;

pub use sqlite::{Database, QueryResult};
pub use pool::ConnectionPool;
pub use models::{User, Email, Endorsement};
