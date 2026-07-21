// email/smtp/message.rs - Email message builder
use base64::Engine;

pub struct EmailMessage {
    pub from: String,
    pub to: String,
    pub subject: String,
    pub body: String,
}

impl EmailMessage {
    pub fn new(from: &str, to: &str, subject: &str, body: &str) -> Self {
        EmailMessage {
            from: from.to_string(),
            to: to.to_string(),
            subject: subject.to_string(),
            body: body.to_string(),
        }
    }
    
    pub fn to_raw(&self) -> String {
        let subject_b64 = base64::engine::general_purpose::STANDARD.encode(self.subject.as_bytes());
        
        format!(
            "From: <{}>\r\n\
             To: <{}>\r\n\
             Subject: =?UTF-8?B?{}?=\r\n\
             MIME-Version: 1.0\r\n\
             Content-Type: text/plain; charset=UTF-8\r\n\
             \r\n\
             {}\r\n\
             .\r\n",
            self.from, self.to, subject_b64, self.body
        )
    }
    
    pub fn generate_endorsement(name: &str, category: &str, code: &str) -> Self {
        let body = format!(
            "Dear {},\n\n\
             I am writing to request your endorsement for my submission to arXiv.\n\n\
             Paper: \"1001 Proofs: A Rigorous Collection with Explicit Assumptions, \n\
             Dependencies, and Verification Boundaries\"\n\n\
             To endorse for {}, please:\n\
             1. Visit: https://arxiv.org/auth/endorse.php\n\
             2. Enter code: {}\n\n\
             Best regards,\n\
             Yuriy Aronov\n\
             apohob5@gmail.com",
            name, category, code
        );
        
        EmailMessage::new("apohob5@gmail.com", "", &format!("Endorsement request - {}", category), &body)
    }
}
