// orchestrator/core/project_manager.rs - Project management
use crate::database::sqlite::connection::Database;
use super::types::{Project, ProjectStatus};
use crate::database::sqlite::error::DbError;

pub struct ProjectManager {
    db: Database,
}

impl ProjectManager {
    pub fn new(db: Database) -> Self {
        ProjectManager { db }
    }
    
    /// Create new project
    pub fn create_project(&self, name: &str, description: &str) -> Result<Project, DbError> {
        let id = self.db.insert_user(name, "", "project")?;
        
        // Insert into projects table
        self.db.execute(&format!(
            "INSERT INTO projects (id, name, description, status) VALUES ({}, '{}', '{}', 'active')",
            id, name, description
        ))?;
        
        Ok(Project {
            id,
            name: name.to_string(),
            description: description.to_string(),
            status: ProjectStatus::Active,
        })
    }
    
    /// Get project by ID
    pub fn get_project(&self, id: i64) -> Result<Option<Project>, DbError> {
        let result = self.db.query(&format!(
            "SELECT id, name, description, status FROM projects WHERE id = {}",
            id
        ))?;
        
        if let Some(row) = result.get(0) {
            let status = match row.get("status").map(|s| s.as_str()) {
                Some("active") => ProjectStatus::Active,
                Some("paused") => ProjectStatus::Paused,
                Some("completed") => ProjectStatus::Completed,
                Some("failed") => ProjectStatus::Failed,
                _ => ProjectStatus::Active,
            };
            
            Ok(Some(Project {
                id: row.get("id").and_then(|s| s.parse().ok()).unwrap_or(0),
                name: row.get("name").cloned().unwrap_or_default(),
                description: row.get("description").cloned().unwrap_or_default(),
                status,
            }))
        } else {
            Ok(None)
        }
    }
    
    /// Get all projects
    pub fn get_all_projects(&self) -> Result<Vec<Project>, DbError> {
        let result = self.db.query("SELECT id, name, description, status FROM projects")?;
        
        let projects = result.rows.into_iter().map(|row| {
            let status = match row.get("status").map(|s| s.as_str()) {
                Some("active") => ProjectStatus::Active,
                Some("paused") => ProjectStatus::Paused,
                Some("completed") => ProjectStatus::Completed,
                Some("failed") => ProjectStatus::Failed,
                _ => ProjectStatus::Active,
            };
            
            Project {
                id: row.get("id").and_then(|s| s.parse().ok()).unwrap_or(0),
                name: row.get("name").cloned().unwrap_or_default(),
                description: row.get("description").cloned().unwrap_or_default(),
                status,
            }
        }).collect();
        
        Ok(projects)
    }
    
    /// Update project status
    pub fn update_status(&self, id: i64, status: ProjectStatus) -> Result<(), DbError> {
        let status_str = match status {
            ProjectStatus::Active => "active",
            ProjectStatus::Paused => "paused",
            ProjectStatus::Completed => "completed",
            ProjectStatus::Failed => "failed",
        };
        
        self.db.execute(&format!(
            "UPDATE projects SET status = '{}', updated_at = CURRENT_TIMESTAMP WHERE id = {}",
            status_str, id
        ))?;
        
        Ok(())
    }
    
    /// Delete project
    pub fn delete_project(&self, id: i64) -> Result<(), DbError> {
        self.db.execute(&format!(
            "DELETE FROM projects WHERE id = {}",
            id
        ))?;
        Ok(())
    }
    
    /// Get project statistics
    pub fn get_stats(&self) -> Result<ProjectStats, DbError> {
        let total = self.db.query("SELECT COUNT(*) as count FROM projects")?;
        let active = self.db.query("SELECT COUNT(*) as count FROM projects WHERE status = 'active'")?;
        let completed = self.db.query("SELECT COUNT(*) as count FROM projects WHERE status = 'completed'")?;
        
        Ok(ProjectStats {
            total: total.rows.first().and_then(|r| r.get("count")).and_then(|s| s.parse().ok()).unwrap_or(0),
            active: active.rows.first().and_then(|r| r.get("count")).and_then(|s| s.parse().ok()).unwrap_or(0),
            completed: completed.rows.first().and_then(|r| r.get("count")).and_then(|s| s.parse().ok()).unwrap_or(0),
        })
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ProjectStats {
    pub total: usize,
    pub active: usize,
    pub completed: usize,
}
