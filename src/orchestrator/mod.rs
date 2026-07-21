// orchestrator/mod.rs - Orchestrator module
pub mod core;
pub mod workers;
pub mod coordinator;

pub use core::project_manager::ProjectManager;
pub use core::branch_manager::BranchManager;
pub use core::chat_manager::ChatManager;
pub use core::task_manager::TaskManager;
pub use core::types::{Project, ProjectStatus, Branch, BranchRole, Task, TaskStatus};
pub use workers::{Worker, WorkerPool, WorkerStatus};
pub use coordinator::{DependencyMonitor, DataExchange, SyncManager};
