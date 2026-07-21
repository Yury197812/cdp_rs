// orchestrator/mod.rs - Orchestrator module
pub mod core;

pub use core::project_manager::ProjectManager;
pub use core::types::{Project, ProjectStatus, Branch, BranchRole};
