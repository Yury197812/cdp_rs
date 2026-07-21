// orchestrator/core/task_manager.rs - Task management
use crate::database::sqlite::connection::Database;
use super::types::{Task, TaskStatus};
use crate::database::sqlite::error::DbError;

pub struct TaskManager {
    db: Database,
}

impl TaskManager {
    pub fn new(db: Database) -> Self {
        TaskManager { db }
    }
    
    /// Create new task
    pub fn create_task(&self, project_id: i64, title: &str, description: &str, priority: i32) -> Result<Task, DbError> {
        self.db.execute(&format!(
            "INSERT INTO tasks (project_id, title, description, priority, status) VALUES ({}, '{}', '{}', {}, 'pending')",
            project_id, title, description.replace('\'', "''"), priority
        ))?;
        
        let id = self.db.query("SELECT last_insert_rowid() as id")?
            .rows.first()
            .and_then(|r| r.get("id"))
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        
        Ok(Task {
            id,
            project_id,
            branch_id: None,
            title: title.to_string(),
            description: description.to_string(),
            priority,
            status: TaskStatus::Pending,
        })
    }
    
    /// Get task by ID
    pub fn get_task(&self, id: i64) -> Result<Option<Task>, DbError> {
        let result = self.db.query(&format!(
            "SELECT id, project_id, branch_id, title, description, priority, status FROM tasks WHERE id = {}",
            id
        ))?;
        
        if let Some(row) = result.get(0) {
            let status = match row.get("status").map(|s| s.as_str()) {
                Some("pending") => TaskStatus::Pending,
                Some("in_progress") => TaskStatus::InProgress,
                Some("completed") => TaskStatus::Completed,
                Some("failed") => TaskStatus::Failed,
                _ => TaskStatus::Pending,
            };
            
            Ok(Some(Task {
                id: row.get("id").and_then(|s| s.parse().ok()).unwrap_or(0),
                project_id: row.get("project_id").and_then(|s| s.parse().ok()).unwrap_or(0),
                branch_id: row.get("branch_id").and_then(|s| s.parse().ok()),
                title: row.get("title").cloned().unwrap_or_default(),
                description: row.get("description").cloned().unwrap_or_default(),
                priority: row.get("priority").and_then(|s| s.parse().ok()).unwrap_or(0),
                status,
            }))
        } else {
            Ok(None)
        }
    }
    
    /// Get all tasks for a project
    pub fn get_tasks(&self, project_id: i64) -> Result<Vec<Task>, DbError> {
        let result = self.db.query(&format!(
            "SELECT id, project_id, branch_id, title, description, priority, status FROM tasks WHERE project_id = {} ORDER BY priority DESC",
            project_id
        ))?;
        
        let tasks = result.rows.into_iter().map(|row| {
            let status = match row.get("status").map(|s| s.as_str()) {
                Some("pending") => TaskStatus::Pending,
                Some("in_progress") => TaskStatus::InProgress,
                Some("completed") => TaskStatus::Completed,
                Some("failed") => TaskStatus::Failed,
                _ => TaskStatus::Pending,
            };
            
            Task {
                id: row.get("id").and_then(|s| s.parse().ok()).unwrap_or(0),
                project_id: row.get("project_id").and_then(|s| s.parse().ok()).unwrap_or(0),
                branch_id: row.get("branch_id").and_then(|s| s.parse().ok()),
                title: row.get("title").cloned().unwrap_or_default(),
                description: row.get("description").cloned().unwrap_or_default(),
                priority: row.get("priority").and_then(|s| s.parse().ok()).unwrap_or(0),
                status,
            }
        }).collect();
        
        Ok(tasks)
    }
    
    /// Assign task to branch
    pub fn assign_task(&self, task_id: i64, branch_id: i64) -> Result<(), DbError> {
        self.db.execute(&format!(
            "UPDATE tasks SET branch_id = {}, status = 'in_progress' WHERE id = {}",
            branch_id, task_id
        ))?;
        Ok(())
    }
    
    /// Complete task
    pub fn complete_task(&self, id: i64) -> Result<(), DbError> {
        self.db.execute(&format!(
            "UPDATE tasks SET status = 'completed' WHERE id = {}",
            id
        ))?;
        Ok(())
    }
    
    /// Fail task
    pub fn fail_task(&self, id: i64) -> Result<(), DbError> {
        self.db.execute(&format!(
            "UPDATE tasks SET status = 'failed' WHERE id = {}",
            id
        ))?;
        Ok(())
    }
    
    /// Get task statistics
    pub fn get_stats(&self, project_id: i64) -> Result<TaskStats, DbError> {
        let total = self.db.query(&format!(
            "SELECT COUNT(*) as count FROM tasks WHERE project_id = {}", project_id
        ))?;
        let pending = self.db.query(&format!(
            "SELECT COUNT(*) as count FROM tasks WHERE project_id = {} AND status = 'pending'", project_id
        ))?;
        let in_progress = self.db.query(&format!(
            "SELECT COUNT(*) as count FROM tasks WHERE project_id = {} AND status = 'in_progress'", project_id
        ))?;
        let completed = self.db.query(&format!(
            "SELECT COUNT(*) as count FROM tasks WHERE project_id = {} AND status = 'completed'", project_id
        ))?;
        
        Ok(TaskStats {
            total: total.rows.first().and_then(|r| r.get("count")).and_then(|s| s.parse().ok()).unwrap_or(0),
            pending: pending.rows.first().and_then(|r| r.get("count")).and_then(|s| s.parse().ok()).unwrap_or(0),
            in_progress: in_progress.rows.first().and_then(|r| r.get("count")).and_then(|s| s.parse().ok()).unwrap_or(0),
            completed: completed.rows.first().and_then(|r| r.get("count")).and_then(|s| s.parse().ok()).unwrap_or(0),
        })
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct TaskStats {
    pub total: usize,
    pub pending: usize,
    pub in_progress: usize,
    pub completed: usize,
}
