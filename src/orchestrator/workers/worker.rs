// orchestrator/workers/worker.rs - Worker implementation (FIXED)
use crate::database::sqlite::connection::Database;
use crate::database::sqlite::error::DbError;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub struct Worker {
    id: String,
    db: Arc<Mutex<Database>>,
    status: WorkerStatus,
    timeout: Duration,
}

#[derive(Debug, Clone, PartialEq)]
pub enum WorkerStatus {
    Idle,
    Working,
    Error(String),
}

impl Worker {
    pub fn new(id: &str, db: Arc<Mutex<Database>>) -> Self {
        Worker {
            id: id.to_string(),
            db,
            status: WorkerStatus::Idle,
            timeout: Duration::from_secs(300), // 5 minutes default
        }
    }
    
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
    
    /// Process a task with error handling
    pub async fn process_task(&mut self, task_id: i64) -> Result<(), DbError> {
        // Validate input
        if task_id <= 0 {
            return Err(DbError::ValidationError("Invalid task ID".to_string()));
        }
        
        self.status = WorkerStatus::Working;
        println!("[Worker {}] Processing task {}", self.id, task_id);
        
        // Get task details with timeout
        let task = {
            let db = self.db.lock().map_err(|e| DbError::QueryError(e.to_string()))?;
            let result = db.query(&format!(
                "SELECT id, title, description FROM tasks WHERE id = {}", task_id
            ))?;
            result.rows.first().cloned()
        };
        
        if let Some(task) = task {
            let title = task.get("title").cloned().unwrap_or_default();
            let description = task.get("description").cloned().unwrap_or_default();
            
            println!("[Worker {}] Task: {}", self.id, title);
            println!("[Worker {}] Description: {}", self.id, description);
            
            // Simulate work with timeout
            match tokio::time::timeout(self.timeout, async {
                tokio::time::sleep(Duration::from_secs(1)).await;
                Ok::<(), DbError>(())
            }).await {
                Ok(Ok(_)) => {
                    // Mark as completed
                    {
                        let db = self.db.lock().map_err(|e| DbError::QueryError(e.to_string()))?;
                        db.execute(&format!(
                            "UPDATE tasks SET status = 'completed' WHERE id = {}", task_id
                        ))?;
                    }
                    self.status = WorkerStatus::Idle;
                    println!("[Worker {}] Task {} completed", self.id, task_id);
                }
                Ok(Err(e)) => {
                    self.status = WorkerStatus::Error(e.to_string());
                    println!("[Worker {}] Task {} failed: {}", self.id, task_id, e);
                    return Err(e);
                }
                Err(_) => {
                    self.status = WorkerStatus::Error("Timeout".to_string());
                    println!("[Worker {}] Task {} timed out", self.id, task_id);
                    return Err(DbError::QueryError("Task timed out".to_string()));
                }
            }
        }
        
        Ok(())
    }
    
    pub fn status(&self) -> &WorkerStatus {
        &self.status
    }
    
    pub fn id(&self) -> &str {
        &self.id
    }
    
    pub fn is_idle(&self) -> bool {
        self.status == WorkerStatus::Idle
    }
}

pub struct WorkerPool {
    workers: Vec<Arc<Mutex<Worker>>>,
}

impl WorkerPool {
    pub fn new(size: usize, db_path: &str) -> Result<Self, DbError> {
        let mut workers = Vec::new();
        for i in 0..size {
            let db = Arc::new(Mutex::new(Database::new(db_path)?));
            workers.push(Arc::new(Mutex::new(Worker::new(&format!("worker-{}", i), db))));
        }
        Ok(WorkerPool { workers })
    }
    
    pub fn get_idle_worker(&self) -> Option<Arc<Mutex<Worker>>> {
        self.workers.iter().find(|w| {
            w.try_lock().map_or(false, |w| w.is_idle())
        }).cloned()
    }
    
    pub fn stats(&self) -> PoolStats {
        let mut idle = 0;
        let mut working = 0;
        let mut error = 0;
        
        for worker in &self.workers {
            if let Ok(w) = worker.try_lock() {
                match &w.status() {
                    WorkerStatus::Idle => idle += 1,
                    WorkerStatus::Working => working += 1,
                    WorkerStatus::Error(_) => error += 1,
                }
            }
        }
        
        PoolStats { total: self.workers.len(), idle, working, error }
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct PoolStats {
    pub total: usize,
    pub idle: usize,
    pub working: usize,
    pub error: usize,
}
