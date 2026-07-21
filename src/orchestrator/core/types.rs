// orchestrator/core/types.rs - Core types for orchestrator

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub status: ProjectStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProjectStatus {
    Active,
    Paused,
    Completed,
    Failed,
}

impl Project {
    pub fn new(name: &str, description: &str) -> Self {
        Project {
            id: 0,
            name: name.to_string(),
            description: description.to_string(),
            status: ProjectStatus::Active,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Branch {
    pub id: i64,
    pub project_id: i64,
    pub name: String,
    pub role: BranchRole,
    pub status: BranchStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BranchRole {
    Coordinator,
    Worker,
    Auditor,
    Synthesizer,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BranchStatus {
    Active,
    Paused,
    Completed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: i64,
    pub project_id: i64,
    pub branch_id: Option<i64>,
    pub title: String,
    pub description: String,
    pub priority: i32,
    pub status: TaskStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}
