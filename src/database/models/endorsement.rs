// database/models/endorsement.rs - Endorsement model
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Endorsement {
    pub id: i64,
    pub user_id: i64,
    pub category: String,
    pub code: String,
    pub status: String,
}

impl Endorsement {
    pub fn new(user_id: i64, category: &str, code: &str) -> Self {
        Endorsement {
            id: 0,
            user_id,
            category: category.to_string(),
            code: code.to_string(),
            status: "pending".to_string(),
        }
    }
    
    pub fn with_id(id: i64, user_id: i64, category: &str, code: &str, status: &str) -> Self {
        Endorsement {
            id,
            user_id,
            category: category.to_string(),
            code: code.to_string(),
            status: status.to_string(),
        }
    }
    
    pub fn approve(&mut self) {
        self.status = "approved".to_string();
    }
    
    pub fn reject(&mut self) {
        self.status = "rejected".to_string();
    }
    
    pub fn is_approved(&self) -> bool {
        self.status == "approved"
    }
}
