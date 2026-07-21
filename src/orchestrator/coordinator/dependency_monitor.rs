// orchestrator/coordinator/dependency_monitor.rs - Dependency tracking
use std::collections::{HashMap, HashSet};

pub struct DependencyMonitor {
    dependencies: HashMap<i64, HashSet<i64>>,
}

impl DependencyMonitor {
    pub fn new() -> Self {
        DependencyMonitor {
            dependencies: HashMap::new(),
        }
    }
    
    /// Add dependency: task_id depends on dependency_id
    pub fn add_dependency(&mut self, task_id: i64, dependency_id: i64) {
        self.dependencies
            .entry(task_id)
            .or_insert_with(HashSet::new)
            .insert(dependency_id);
    }
    
    /// Remove dependency
    pub fn remove_dependency(&mut self, task_id: i64, dependency_id: i64) {
        if let Some(deps) = self.dependencies.get_mut(&task_id) {
            deps.remove(&dependency_id);
        }
    }
    
    /// Check if task has unresolved dependencies
    pub fn has_unresolved(&self, task_id: i64, completed: &HashSet<i64>) -> bool {
        if let Some(deps) = self.dependencies.get(&task_id) {
            deps.iter().any(|d| !completed.contains(d))
        } else {
            false
        }
    }
    
    /// Get all dependencies for a task
    pub fn get_dependencies(&self, task_id: i64) -> Vec<i64> {
        self.dependencies
            .get(&task_id)
            .map(|deps| deps.iter().cloned().collect())
            .unwrap_or_default()
    }
    
    /// Get tasks that depend on a given task
    pub fn get_dependents(&self, task_id: i64) -> Vec<i64> {
        self.dependencies
            .iter()
            .filter(|(_, deps)| deps.contains(&task_id))
            .map(|(id, _)| *id)
            .collect()
    }
    
    /// Check for circular dependencies
    pub fn has_cycle(&self) -> bool {
        let mut visited = HashSet::new();
        let mut path = HashSet::new();
        
        for task_id in self.dependencies.keys() {
            if self.has_cycle_dfs(*task_id, &mut visited, &mut path) {
                return true;
            }
        }
        false
    }
    
    fn has_cycle_dfs(&self, task_id: i64, visited: &mut HashSet<i64>, path: &mut HashSet<i64>) -> bool {
        if path.contains(&task_id) {
            return true;
        }
        if visited.contains(&task_id) {
            return false;
        }
        
        visited.insert(task_id);
        path.insert(task_id);
        
        if let Some(deps) = self.dependencies.get(&task_id) {
            for dep in deps {
                if self.has_cycle_dfs(*dep, visited, path) {
                    return true;
                }
            }
        }
        
        path.remove(&task_id);
        false
    }
}
