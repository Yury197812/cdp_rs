// orchestrator/dashboard/stats.rs - Dashboard statistics
use crate::orchestrator::core::ProjectManager;
use crate::database::sqlite::connection::Database;

pub struct DashboardStats {
    project_manager: ProjectManager,
}

impl DashboardStats {
    pub fn new(db: Database) -> Self {
        DashboardStats {
            project_manager: ProjectManager::new(db),
        }
    }
    
    /// Get all statistics
    pub fn get_all_stats(&self) -> Result<AllStats, String> {
        let project_stats = self.project_manager.get_stats()
            .map_err(|e| e.to_string())?;
        
        Ok(AllStats {
            projects: ProjectStats {
                total: project_stats.total,
                active: project_stats.active,
                completed: project_stats.completed,
            },
            tasks: TaskStats {
                total: 0,
                pending: 0,
                in_progress: 0,
                completed: 0,
            },
            workers: WorkerStats {
                total: 0,
                idle: 0,
                working: 0,
            },
        })
    }
}

#[derive(serde::Serialize)]
pub struct AllStats {
    pub projects: ProjectStats,
    pub tasks: TaskStats,
    pub workers: WorkerStats,
}

#[derive(serde::Serialize)]
pub struct ProjectStats {
    pub total: usize,
    pub active: usize,
    pub completed: usize,
}

#[derive(serde::Serialize)]
pub struct TaskStats {
    pub total: usize,
    pub pending: usize,
    pub in_progress: usize,
    pub completed: usize,
}

#[derive(serde::Serialize)]
pub struct WorkerStats {
    pub total: usize,
    pub idle: usize,
    pub working: usize,
}
