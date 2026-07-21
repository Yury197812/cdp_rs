// tests/orchestrator_tests.rs - Orchestrator module tests
#[cfg(test)]
mod tests {
    use cdp_rs::database::sqlite::connection::Database;
    use cdp_rs::orchestrator::core::project_manager::ProjectManager;
    use cdp_rs::orchestrator::core::branch_manager::BranchManager;
    use cdp_rs::orchestrator::core::task_manager::TaskManager;
    use cdp_rs::orchestrator::core::types::{ProjectStatus, BranchRole, TaskStatus};
    use cdp_rs::orchestrator::coordinator::dependency_monitor::DependencyMonitor;
    use cdp_rs::orchestrator::coordinator::data_exchange::DataExchange;
    
    #[test]
    fn test_project_create() {
        let db = Database::create(":memory:").unwrap();
        let pm = ProjectManager::new(db);
        let project = pm.create_project("Test Project", "Description").unwrap();
        assert!(project.id > 0);
        assert_eq!(project.name, "Test Project");
    }
    
    #[test]
    fn test_project_get() {
        let db = Database::create(":memory:").unwrap();
        let pm = ProjectManager::new(db);
        let project = pm.create_project("Test Project", "Description").unwrap();
        let retrieved = pm.get_project(project.id).unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "Test Project");
    }
    
    #[test]
    fn test_project_status_update() {
        let db = Database::create(":memory:").unwrap();
        let pm = ProjectManager::new(db);
        let project = pm.create_project("Test Project", "Description").unwrap();
        pm.update_status(project.id, ProjectStatus::Completed).unwrap();
        let retrieved = pm.get_project(project.id).unwrap().unwrap();
        assert_eq!(retrieved.status, ProjectStatus::Completed);
    }
    
    #[test]
    fn test_branch_create() {
        let db = Database::create(":memory:").unwrap();
        let pm = ProjectManager::new(Database::create(":memory:").unwrap());
        let bm = BranchManager::new(db);
        let project = pm.create_project("Test", "Desc").unwrap();
        let branch = bm.create_branch(project.id, "Analysis", BranchRole::Worker).unwrap();
        assert!(branch.id > 0);
        assert_eq!(branch.name, "Analysis");
    }
    
    #[test]
    fn test_task_create() {
        let db = Database::create(":memory:").unwrap();
        let pm = ProjectManager::new(Database::create(":memory:").unwrap());
        let tm = TaskManager::new(db);
        let project = pm.create_project("Test", "Desc").unwrap();
        let task = tm.create_task(project.id, "Task 1", "Description", 10).unwrap();
        assert!(task.id > 0);
        assert_eq!(task.title, "Task 1");
    }
    
    #[test]
    fn test_task_status() {
        let db = Database::create(":memory:").unwrap();
        let pm = ProjectManager::new(Database::create(":memory:").unwrap());
        let tm = TaskManager::new(db);
        let project = pm.create_project("Test", "Desc").unwrap();
        let task = tm.create_task(project.id, "Task 1", "Desc", 10).unwrap();
        
        tm.complete_task(task.id).unwrap();
        let retrieved = tm.get_task(task.id).unwrap().unwrap();
        assert_eq!(retrieved.status, TaskStatus::Completed);
    }
    
    #[test]
    fn test_dependency_monitor() {
        let mut dm = DependencyMonitor::new();
        dm.add_dependency(2, 1);
        dm.add_dependency(3, 1);
        dm.add_dependency(3, 2);
        
        let completed = [1].iter().cloned().collect();
        assert!(!dm.has_unresolved(2, &completed));
        assert!(dm.has_unresolved(3, &completed));
    }
    
    #[test]
    fn test_dependency_cycle() {
        let mut dm = DependencyMonitor::new();
        dm.add_dependency(1, 2);
        dm.add_dependency(2, 3);
        dm.add_dependency(3, 1);
        
        assert!(dm.has_cycle());
    }
    
    #[test]
    fn test_data_exchange() {
        let mut de = DataExchange::new();
        de.store("key1", "value1");
        assert_eq!(de.retrieve("key1"), Some(&"value1".to_string()));
        assert!(de.has("key1"));
        de.remove("key1");
        assert!(!de.has("key1"));
    }
}
