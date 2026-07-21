// database/models/user.rs - User model
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct User {
    pub id: i64,
    pub name: String,
    pub email: String,
    pub category: String,
}

impl User {
    pub fn new(name: &str, email: &str, category: &str) -> Self {
        User {
            id: 0,
            name: name.to_string(),
            email: email.to_string(),
            category: category.to_string(),
        }
    }
    
    pub fn with_id(id: i64, name: &str, email: &str, category: &str) -> Self {
        User {
            id,
            name: name.to_string(),
            email: email.to_string(),
            category: category.to_string(),
        }
    }
}
