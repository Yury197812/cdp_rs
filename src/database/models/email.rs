// database/models/email.rs - Email model
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Email {
    pub id: i64,
    pub from: String,
    pub to: String,
    pub subject: String,
    pub body: String,
}

impl Email {
    pub fn new(from: &str, to: &str, subject: &str, body: &str) -> Self {
        Email {
            id: 0,
            from: from.to_string(),
            to: to.to_string(),
            subject: subject.to_string(),
            body: body.to_string(),
        }
    }
    
    pub fn with_id(id: i64, from: &str, to: &str, subject: &str, body: &str) -> Self {
        Email {
            id,
            from: from.to_string(),
            to: to.to_string(),
            subject: subject.to_string(),
            body: body.to_string(),
        }
    }
}
