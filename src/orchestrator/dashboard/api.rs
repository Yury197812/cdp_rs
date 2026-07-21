// orchestrator/dashboard/api.rs - REST API endpoints
use crate::orchestrator::core::{ProjectManager, BranchManager, TaskManager};
use crate::database::sqlite::connection::Database;
use crate::database::sqlite::error::DbError;
use crate::database::models::User;

pub struct DashboardApi {
    project_manager: ProjectManager,
    branch_manager: BranchManager,
    task_manager: TaskManager,
    db: Database,
}

impl DashboardApi {
    pub fn new(db_path: &str) -> Result<Self, DbError> {
        let db = Database::new(db_path)?;
        Ok(DashboardApi {
            project_manager: ProjectManager::new(Database::new(db_path)?),
            branch_manager: BranchManager::new(Database::new(db_path)?),
            task_manager: TaskManager::new(Database::new(db_path)?),
            db,
        })
    }
    
    /// Get dashboard overview
    pub fn get_overview(&self) -> Result<DashboardOverview, String> {
        let projects = self.project_manager.get_all_projects()
            .map_err(|e| e.to_string())?;
        
        let stats = self.project_manager.get_stats()
            .map_err(|e| e.to_string())?;
        
        Ok(DashboardOverview {
            projects: projects.len(),
            active: stats.active,
            completed: stats.completed,
        })
    }
    
    /// Get project details
    pub fn get_project(&self, id: i64) -> Result<serde_json::Value, String> {
        let project = self.project_manager.get_project(id)
            .map_err(|e| e.to_string())?;
        
        match project {
            Some(p) => Ok(serde_json::to_value(p).unwrap_or_default()),
            None => Err("Project not found".to_string()),
        }
    }
    
    /// Get branches for project
    pub fn get_branches(&self, project_id: i64) -> Result<serde_json::Value, String> {
        let branches = self.branch_manager.get_branches(project_id)
            .map_err(|e| e.to_string())?;
        
        Ok(serde_json::to_value(branches).unwrap_or_default())
    }
    
    /// Get tasks for project
    pub fn get_tasks(&self, project_id: i64) -> Result<serde_json::Value, String> {
        let tasks = self.task_manager.get_tasks(project_id)
            .map_err(|e| e.to_string())?;
        
        Ok(serde_json::to_value(tasks).unwrap_or_default())
    }
    
    // User methods
    pub fn get_all_users(&self) -> Result<Vec<User>, String> {
        self.db.get_all_users().map_err(|e| e.to_string())
    }
    
    pub fn get_user(&self, id: i64) -> Result<Option<User>, String> {
        self.db.get_user(id).map_err(|e| e.to_string())
    }
    
    pub fn create_user(&self, name: &str, email: &str, category: &str) -> Result<User, String> {
        let id = self.db.insert_user(name, email, category)
            .map_err(|e| e.to_string())?;
        Ok(User::with_id(id, name, email, category))
    }
    
    pub fn update_user(&self, id: i64, name: &str, email: &str) -> Result<(), String> {
        self.db.execute(&format!(
            "UPDATE users SET name = '{}', email = '{}' WHERE id = {}",
            name, email, id
        )).map_err(|e| e.to_string())?;
        Ok(())
    }
    
    pub fn delete_user(&self, id: i64) -> Result<(), String> {
        self.db.execute(&format!(
            "DELETE FROM users WHERE id = {}",
            id
        )).map_err(|e| e.to_string())?;
        Ok(())
    }
}

#[derive(serde::Serialize)]
pub struct DashboardOverview {
    pub projects: usize,
    pub active: usize,
    pub completed: usize,
}
