// database/models/email.rs - Email model

#[derive(Debug, Clone)]
pub struct Email {
    pub id: i64,
    pub from: String,
    pub to: String,
    pub subject: String,
    pub body: String,
    pub timestamp: String,
}

impl Email {
    pub fn new(id: i64, from: &str, to: &str, subject: &str, body: &str) -> Self {
        Email {
            id,
            from: from.to_string(),
            to: to.to_string(),
            subject: subject.to_string(),
            body: body.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }
    
    pub fn to_sql(&self) -> String {
        format!(
            "INSERT INTO emails (from_addr, to_addr, subject, body, timestamp) VALUES ('{}', '{}', '{}', '{}', '{}')",
            self.from, self.to, self.subject, self.body, self.timestamp
        )
    }
}
