// orchestrator/workers/worker.rs - Worker implementation
use crate::database::sqlite::connection::Database;
use crate::orchestrator::core::task_manager::TaskManager;
use crate::database::sqlite::error::DbError;

pub struct Worker {
    id: String,
    task_manager: TaskManager,
    status: WorkerStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub enum WorkerStatus {
    Idle,
    Working,
    Error,
}

impl Worker {
    pub fn new(id: &str, db: Database) -> Self {
        Worker {
            id: id.to_string(),
            task_manager: TaskManager::new(db),
            status: WorkerStatus::Idle,
        }
    }
    
    /// Process a task
    pub async fn process_task(&mut self, task_id: i64) -> Result<(), DbError> {
        self.status = WorkerStatus::Working;
        
        println!("[Worker {}] Processing task {}", self.id, task_id);
        
        // Get task details
        let task = self.task_manager.get_task(task_id)?;
        
        if let Some(task) = task {
            println!("[Worker {}] Task: {}", self.id, task.title);
            println!("[Worker {}] Description: {}", self.id, task.description);
            
            // Simulate work
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            
            // Mark as completed
            self.task_manager.complete_task(task_id)?;
            self.status = WorkerStatus::Idle;
            
            println!("[Worker {}] Task {} completed", self.id, task_id);
        }
        
        Ok(())
    }
    
    /// Get worker status
    pub fn status(&self) -> &WorkerStatus {
        &self.status
    }
    
    /// Get worker ID
    pub fn id(&self) -> &str {
        &self.id
    }
}

pub struct WorkerPool {
    workers: Vec<Worker>,
}

impl WorkerPool {
    pub fn new(size: usize, db_path: &str) -> Result<Self, crate::database::sqlite::error::DbError> {
        let mut workers = Vec::new();
        for i in 0..size {
            let db = Database::new(db_path)?;
            workers.push(Worker::new(&format!("worker-{}", i), db));
        }
        Ok(WorkerPool { workers })
    }
    
    /// Get idle worker
    pub fn get_idle_worker(&mut self) -> Option<&mut Worker> {
        self.workers.iter_mut().find(|w| *w.status() == WorkerStatus::Idle)
    }
    
    /// Get pool statistics
    pub fn stats(&self) -> PoolStats {
        let idle = self.workers.iter().filter(|w| *w.status() == WorkerStatus::Idle).count();
        let working = self.workers.iter().filter(|w| *w.status() == WorkerStatus::Working).count();
        let error = self.workers.iter().filter(|w| *w.status() == WorkerStatus::Error).count();
        
        PoolStats {
            total: self.workers.len(),
            idle,
            working,
            error,
        }
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct PoolStats {
    pub total: usize,
    pub idle: usize,
    pub working: usize,
    pub error: usize,
}
