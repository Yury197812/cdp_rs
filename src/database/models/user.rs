// database/models/user.rs - User model

#[derive(Debug, Clone)]
pub struct User {
    pub id: i64,
    pub name: String,
    pub email: String,
    pub category: String,
}

impl User {
    pub fn new(id: i64, name: &str, email: &str, category: &str) -> Self {
        User {
            id,
            name: name.to_string(),
            email: email.to_string(),
            category: category.to_string(),
        }
    }
    
    pub fn to_sql(&self) -> String {
        format!(
            "INSERT INTO users (name, email, category) VALUES ('{}', '{}', '{}')",
            self.name, self.email, self.category
        )
    }
}
