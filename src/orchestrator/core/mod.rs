// orchestrator/core/mod.rs - Core orchestrator module
pub mod types;
pub mod project_manager;
pub mod branch_manager;
pub mod chat_manager;
pub mod task_manager;

pub use types::*;
pub use project_manager::ProjectManager;
pub use branch_manager::BranchManager;
pub use chat_manager::ChatManager;
pub use task_manager::TaskManager;
