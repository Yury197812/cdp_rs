// orchestrator/core/branch_manager.rs - Branch management
use crate::database::sqlite::connection::Database;
use super::types::{Branch, BranchRole, BranchStatus};
use crate::database::sqlite::error::DbError;

pub struct BranchManager {
    db: Database,
}

impl BranchManager {
    pub fn new(db: Database) -> Self {
        BranchManager { db }
    }
    
    /// Create new branch
    pub fn create_branch(&self, project_id: i64, name: &str, role: BranchRole) -> Result<Branch, DbError> {
        let role_str = match role {
            BranchRole::Coordinator => "coordinator",
            BranchRole::Worker => "worker",
            BranchRole::Auditor => "auditor",
            BranchRole::Synthesizer => "synthesizer",
        };
        
        self.db.execute(&format!(
            "INSERT INTO branches (project_id, name, role, status) VALUES ({}, '{}', '{}', 'active')",
            project_id, name, role_str
        ))?;
        
        let id = self.db.query("SELECT last_insert_rowid() as id")?
            .rows.first()
            .and_then(|r| r.get("id"))
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        
        Ok(Branch {
            id,
            project_id,
            name: name.to_string(),
            role,
            status: BranchStatus::Active,
        })
    }
    
    /// Get branch by ID
    pub fn get_branch(&self, id: i64) -> Result<Option<Branch>, DbError> {
        let result = self.db.query(&format!(
            "SELECT id, project_id, name, role, status FROM branches WHERE id = {}",
            id
        ))?;
        
        if let Some(row) = result.get(0) {
            let role = match row.get("role").map(|s| s.as_str()) {
                Some("coordinator") => BranchRole::Coordinator,
                Some("worker") => BranchRole::Worker,
                Some("auditor") => BranchRole::Auditor,
                Some("synthesizer") => BranchRole::Synthesizer,
                _ => BranchRole::Worker,
            };
            
            let status = match row.get("status").map(|s| s.as_str()) {
                Some("active") => BranchStatus::Active,
                Some("paused") => BranchStatus::Paused,
                Some("completed") => BranchStatus::Completed,
                _ => BranchStatus::Active,
            };
            
            Ok(Some(Branch {
                id: row.get("id").and_then(|s| s.parse().ok()).unwrap_or(0),
                project_id: row.get("project_id").and_then(|s| s.parse().ok()).unwrap_or(0),
                name: row.get("name").cloned().unwrap_or_default(),
                role,
                status,
            }))
        } else {
            Ok(None)
        }
    }
    
    /// Get all branches for a project
    pub fn get_branches(&self, project_id: i64) -> Result<Vec<Branch>, DbError> {
        let result = self.db.query(&format!(
            "SELECT id, project_id, name, role, status FROM branches WHERE project_id = {}",
            project_id
        ))?;
        
        let branches = result.rows.into_iter().map(|row| {
            let role = match row.get("role").map(|s| s.as_str()) {
                Some("coordinator") => BranchRole::Coordinator,
                Some("worker") => BranchRole::Worker,
                Some("auditor") => BranchRole::Auditor,
                Some("synthesizer") => BranchRole::Synthesizer,
                _ => BranchRole::Worker,
            };
            
            let status = match row.get("status").map(|s| s.as_str()) {
                Some("active") => BranchStatus::Active,
                Some("paused") => BranchStatus::Paused,
                Some("completed") => BranchStatus::Completed,
                _ => BranchStatus::Active,
            };
            
            Branch {
                id: row.get("id").and_then(|s| s.parse().ok()).unwrap_or(0),
                project_id: row.get("project_id").and_then(|s| s.parse().ok()).unwrap_or(0),
                name: row.get("name").cloned().unwrap_or_default(),
                role,
                status,
            }
        }).collect();
        
        Ok(branches)
    }
    
    /// Update branch status
    pub fn update_status(&self, id: i64, status: BranchStatus) -> Result<(), DbError> {
        let status_str = match status {
            BranchStatus::Active => "active",
            BranchStatus::Paused => "paused",
            BranchStatus::Completed => "completed",
        };
        
        self.db.execute(&format!(
            "UPDATE branches SET status = '{}' WHERE id = {}",
            status_str, id
        ))?;
        
        Ok(())
    }
    
    /// Delete branch
    pub fn delete_branch(&self, id: i64) -> Result<(), DbError> {
        self.db.execute(&format!(
            "DELETE FROM branches WHERE id = {}",
            id
        ))?;
        Ok(())
    }
}
