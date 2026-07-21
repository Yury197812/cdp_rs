// orchestrator/core/chat_manager.rs - Chat management
use crate::database::sqlite::connection::Database;
use crate::database::sqlite::error::DbError;

pub struct ChatManager {
    db: Database,
}

impl ChatManager {
    pub fn new(db: Database) -> Self {
        ChatManager { db }
    }
    
    /// Create new chat for a branch
    pub fn create_chat(&self, branch_id: i64, role: &str) -> Result<i64, DbError> {
        self.db.execute(&format!(
            "INSERT INTO chats (branch_id, role, status) VALUES ({}, '{}', 'active')",
            branch_id, role
        ))?;
        
        let id = self.db.query("SELECT last_insert_rowid() as id")?
            .rows.first()
            .and_then(|r| r.get("id"))
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        
        Ok(id)
    }
    
    /// Get chat by ID
    pub fn get_chat(&self, id: i64) -> Result<Option<ChatInfo>, DbError> {
        let result = self.db.query(&format!(
            "SELECT id, branch_id, role, status FROM chats WHERE id = {}",
            id
        ))?;
        
        if let Some(row) = result.get(0) {
            Ok(Some(ChatInfo {
                id: row.get("id").and_then(|s| s.parse().ok()).unwrap_or(0),
                branch_id: row.get("branch_id").and_then(|s| s.parse().ok()).unwrap_or(0),
                role: row.get("role").cloned().unwrap_or_default(),
                status: row.get("status").cloned().unwrap_or_default(),
            }))
        } else {
            Ok(None)
        }
    }
    
    /// Get all chats for a branch
    pub fn get_chats(&self, branch_id: i64) -> Result<Vec<ChatInfo>, DbError> {
        let result = self.db.query(&format!(
            "SELECT id, branch_id, role, status FROM chats WHERE branch_id = {}",
            branch_id
        ))?;
        
        let chats = result.rows.into_iter().map(|row| {
            ChatInfo {
                id: row.get("id").and_then(|s| s.parse().ok()).unwrap_or(0),
                branch_id: row.get("branch_id").and_then(|s| s.parse().ok()).unwrap_or(0),
                role: row.get("role").cloned().unwrap_or_default(),
                status: row.get("status").cloned().unwrap_or_default(),
            }
        }).collect();
        
        Ok(chats)
    }
    
    /// Send message to chat
    pub fn send_message(&self, chat_id: i64, role: &str, content: &str) -> Result<i64, DbError> {
        self.db.execute(&format!(
            "INSERT INTO messages (chat_id, role, content) VALUES ({}, '{}', '{}')",
            chat_id, role, content.replace('\'', "''")
        ))?;
        
        let id = self.db.query("SELECT last_insert_rowid() as id")?
            .rows.first()
            .and_then(|r| r.get("id"))
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        
        Ok(id)
    }
    
    /// Get messages for a chat
    pub fn get_messages(&self, chat_id: i64) -> Result<Vec<Message>, DbError> {
        let result = self.db.query(&format!(
            "SELECT id, role, content, timestamp FROM messages WHERE chat_id = {} ORDER BY timestamp",
            chat_id
        ))?;
        
        let messages = result.rows.into_iter().map(|row| {
            Message {
                id: row.get("id").and_then(|s| s.parse().ok()).unwrap_or(0),
                role: row.get("role").cloned().unwrap_or_default(),
                content: row.get("content").cloned().unwrap_or_default(),
                timestamp: row.get("timestamp").cloned().unwrap_or_default(),
            }
        }).collect();
        
        Ok(messages)
    }
    
    /// Update chat status
    pub fn update_status(&self, id: i64, status: &str) -> Result<(), DbError> {
        self.db.execute(&format!(
            "UPDATE chats SET status = '{}' WHERE id = {}",
            status, id
        ))?;
        Ok(())
    }
    
    /// Delete chat
    pub fn delete_chat(&self, id: i64) -> Result<(), DbError> {
        self.db.execute(&format!(
            "DELETE FROM messages WHERE chat_id = {}",
            id
        ))?;
        self.db.execute(&format!(
            "DELETE FROM chats WHERE id = {}",
            id
        ))?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ChatInfo {
    pub id: i64,
    pub branch_id: i64,
    pub role: String,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct Message {
    pub id: i64,
    pub role: String,
    pub content: String,
    pub timestamp: String,
}
