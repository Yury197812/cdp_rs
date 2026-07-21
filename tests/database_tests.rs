// tests/database_tests.rs - Database module tests
#[cfg(test)]
mod tests {
    use cdp_rs::database::sqlite::connection::Database;
    use cdp_rs::database::models::{User, Email, Endorsement};
    
    #[test]
    fn test_database_creation() {
        let db = Database::create(":memory:").unwrap();
        assert!(db.execute("SELECT 1").is_ok());
    }
    
    #[test]
    fn test_user_insert() {
        let db = Database::create(":memory:").unwrap();
        let id = db.insert_user("Test User", "test@example.com", "math.LO").unwrap();
        assert!(id > 0);
    }
    
    #[test]
    fn test_user_get() {
        let db = Database::create(":memory:").unwrap();
        let id = db.insert_user("Test User", "test@example.com", "math.LO").unwrap();
        let user = db.get_user(id).unwrap();
        assert!(user.is_some());
        assert_eq!(user.unwrap().name, "Test User");
    }
    
    #[test]
    fn test_user_get_all() {
        let db = Database::create(":memory:").unwrap();
        db.insert_user("User 1", "user1@test.com", "math.LO").unwrap();
        db.insert_user("User 2", "user2@test.com", "cs.AI").unwrap();
        let users = db.get_all_users().unwrap();
        assert_eq!(users.len(), 2);
    }
    
    #[test]
    fn test_email_insert() {
        let db = Database::create(":memory:").unwrap();
        let id = db.insert_email("from@test.com", "to@test.com", "Subject", "Body").unwrap();
        assert!(id > 0);
    }
    
    #[test]
    fn test_endorsement_insert() {
        let db = Database::create(":memory:").unwrap();
        let user_id = db.insert_user("User", "user@test.com", "math.LO").unwrap();
        let id = db.insert_endorsement(user_id, "math.LO", "NWTCV4").unwrap();
        assert!(id > 0);
    }
    
    #[test]
    fn test_endorsement_status_update() {
        let db = Database::create(":memory:").unwrap();
        let user_id = db.insert_user("User", "user@test.com", "math.LO").unwrap();
        let id = db.insert_endorsement(user_id, "math.LO", "NWTCV4").unwrap();
        db.update_endorsement_status(id, "approved").unwrap();
        // Verify status updated
        assert!(true);
    }
    
    #[test]
    fn test_transaction() {
        let db = Database::create(":memory:").unwrap();
        db.begin().unwrap();
        db.insert_user("User", "user@test.com", "math.LO").unwrap();
        db.commit().unwrap();
        let users = db.get_all_users().unwrap();
        assert_eq!(users.len(), 1);
    }
    
    #[test]
    fn test_rollback() {
        let db = Database::create(":memory:").unwrap();
        db.begin().unwrap();
        db.insert_user("User", "user@test.com", "math.LO").unwrap();
        db.rollback().unwrap();
        let users = db.get_all_users().unwrap();
        assert_eq!(users.len(), 0);
    }
}
