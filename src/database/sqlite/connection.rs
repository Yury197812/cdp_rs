// database/sqlite/connection.rs - SQLite connection with rusqlite
use rusqlite::{Connection, params};
use std::path::Path;
use super::query::QueryResult;
use super::error::DbError;

pub struct Database {
    conn: Connection,
    path: String,
}

impl Database {
    /// Open existing database
    pub fn new(path: &str) -> Result<Self, DbError> {
        if !Path::new(path).exists() {
            return Err(DbError::ConnectionError(format!("File not found: {}", path)));
        }
        
        let conn = Connection::open(path)
            .map_err(|e| DbError::ConnectionError(e.to_string()))?;
        
        // Enable WAL mode for better concurrency
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")
            .map_err(|e| DbError::QueryError(e.to_string()))?;
        
        println!("[DB] Connected to: {}", path);
        Ok(Database { conn, path: path.to_string() })
    }
    
    /// Create new database with schema
    pub fn create(path: &str) -> Result<Self, DbError> {
        let conn = Connection::open(path)
            .map_err(|e| DbError::ConnectionError(e.to_string()))?;
        
        // Enable WAL mode
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")
            .map_err(|e| DbError::QueryError(e.to_string()))?;
        
        // Create tables
        conn.execute_batch(SCHEMA)
            .map_err(|e| DbError::QueryError(e.to_string()))?;
        
        println!("[DB] Created: {}", path);
        Ok(Database { conn, path: path.to_string() })
    }
    
    /// Execute SQL command
    pub fn execute(&self, sql: &str) -> Result<QueryResult, DbError> {
        self.conn.execute_batch(sql)
            .map_err(|e| DbError::QueryError(e.to_string()))?;
        Ok(QueryResult::new())
    }
    
    /// Query data
    pub fn query(&self, sql: &str) -> Result<QueryResult, DbError> {
        let mut stmt = self.conn.prepare(sql)
            .map_err(|e| DbError::QueryError(e.to_string()))?;
        
        let mut result = QueryResult::new();
        let columns: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();
        
        let rows = stmt.query_map([], |row| {
            let mut map = std::collections::HashMap::new();
            for (i, col) in columns.iter().enumerate() {
                let value: String = row.get(i).unwrap_or_default();
                map.insert(col.clone(), value);
            }
            Ok(map)
        }).map_err(|e| DbError::QueryError(e.to_string()))?;
        
        for row in rows {
            if let Ok(map) = row {
                result.push(map);
            }
        }
        
        Ok(result)
    }
    
    /// Insert user
    pub fn insert_user(&self, name: &str, email: &str, category: &str) -> Result<i64, DbError> {
        self.conn.execute(
            "INSERT INTO users (name, email, category) VALUES (?1, ?2, ?3)",
            params![name, email, category],
        ).map_err(|e| DbError::QueryError(e.to_string()))?;
        
        Ok(self.conn.last_insert_rowid())
    }
    
    /// Get user by ID
    pub fn get_user(&self, id: i64) -> Result<Option<crate::database::User>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, email, category FROM users WHERE id = ?1"
        ).map_err(|e| DbError::QueryError(e.to_string()))?;
        
        let mut rows = stmt.query_map(params![id], |row| {
            Ok(crate::database::User {
                id: row.get(0)?,
                name: row.get(1)?,
                email: row.get(2)?,
                category: row.get(3)?,
            })
        }).map_err(|e| DbError::QueryError(e.to_string()))?;
        
        rows.next().transpose().map_err(|e| DbError::QueryError(e.to_string()))
    }
    
    /// Get all users
    pub fn get_all_users(&self) -> Result<Vec<crate::database::User>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, email, category FROM users"
        ).map_err(|e| DbError::QueryError(e.to_string()))?;
        
        let rows = stmt.query_map([], |row| {
            Ok(crate::database::User {
                id: row.get(0)?,
                name: row.get(1)?,
                email: row.get(2)?,
                category: row.get(3)?,
            })
        }).map_err(|e| DbError::QueryError(e.to_string()))?;
        
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| DbError::QueryError(e.to_string()))
    }
    
    /// Insert email
    pub fn insert_email(&self, from: &str, to: &str, subject: &str, body: &str) -> Result<i64, DbError> {
        self.conn.execute(
            "INSERT INTO emails (from_addr, to_addr, subject, body) VALUES (?1, ?2, ?3, ?4)",
            params![from, to, subject, body],
        ).map_err(|e| DbError::QueryError(e.to_string()))?;
        
        Ok(self.conn.last_insert_rowid())
    }
    
    /// Insert endorsement
    pub fn insert_endorsement(&self, user_id: i64, category: &str, code: &str) -> Result<i64, DbError> {
        self.conn.execute(
            "INSERT INTO endorsements (user_id, category, code, status) VALUES (?1, ?2, ?3, 'pending')",
            params![user_id, category, code],
        ).map_err(|e| DbError::QueryError(e.to_string()))?;
        
        Ok(self.conn.last_insert_rowid())
    }
    
    /// Update endorsement status
    pub fn update_endorsement_status(&self, id: i64, status: &str) -> Result<(), DbError> {
        self.conn.execute(
            "UPDATE endorsements SET status = ?1 WHERE id = ?2",
            params![status, id],
        ).map_err(|e| DbError::QueryError(e.to_string()))?;
        Ok(())
    }
    
    /// Begin transaction
    pub fn begin(&self) -> Result<(), DbError> {
        self.conn.execute_batch("BEGIN TRANSACTION")
            .map_err(|e| DbError::QueryError(e.to_string()))?;
        Ok(())
    }
    
    /// Commit transaction
    pub fn commit(&self) -> Result<(), DbError> {
        self.conn.execute_batch("COMMIT")
            .map_err(|e| DbError::QueryError(e.to_string()))?;
        Ok(())
    }
    
    /// Rollback transaction
    pub fn rollback(&self) -> Result<(), DbError> {
        self.conn.execute_batch("ROLLBACK")
            .map_err(|e| DbError::QueryError(e.to_string()))?;
        Ok(())
    }
}

/// Database schema
const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    username TEXT NOT NULL UNIQUE,
    email TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    role TEXT NOT NULL DEFAULT 'user',
    category TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS emails (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    from_addr TEXT NOT NULL,
    to_addr TEXT NOT NULL,
    subject TEXT NOT NULL,
    body TEXT,
    sent_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS endorsements (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL,
    category TEXT NOT NULL,
    code TEXT NOT NULL,
    status TEXT DEFAULT 'pending',
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id)
);

CREATE TABLE IF NOT EXISTS projects (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    description TEXT,
    status TEXT DEFAULT 'active',
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS branches (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    role TEXT NOT NULL,
    status TEXT DEFAULT 'active',
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (project_id) REFERENCES projects(id)
);

CREATE TABLE IF NOT EXISTS tasks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id INTEGER NOT NULL,
    branch_id INTEGER,
    title TEXT NOT NULL,
    description TEXT,
    priority INTEGER DEFAULT 0,
    status TEXT DEFAULT 'pending',
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (project_id) REFERENCES projects(id),
    FOREIGN KEY (branch_id) REFERENCES branches(id)
);
";
