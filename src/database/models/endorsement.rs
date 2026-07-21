// database/models/endorsement.rs - Endorsement model

#[derive(Debug, Clone)]
pub struct EndorsementRecord {
    pub id: i64,
    pub user_id: i64,
    pub category: String,
    pub code: String,
    pub status: String,
    pub created_at: String,
}

impl EndorsementRecord {
    pub fn new(id: i64, user_id: i64, category: &str, code: &str) -> Self {
        EndorsementRecord {
            id,
            user_id,
            category: category.to_string(),
            code: code.to_string(),
            status: "pending".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }
    
    pub fn to_sql(&self) -> String {
        format!(
            "INSERT INTO endorsements (user_id, category, code, status) VALUES ({}, '{}', '{}', '{}')",
            self.user_id, self.category, self.code, self.status
        )
    }
    
    pub fn approve(&mut self) {
        self.status = "approved".to_string();
    }
    
    pub fn reject(&mut self) {
        self.status = "rejected".to_string();
    }
}
