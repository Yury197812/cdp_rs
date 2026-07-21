// email/validator/dns.rs - DNS-based email validation

pub fn validate_email(email: &str) -> EmailValidation {
    let parts: Vec<&str> = email.split('@').collect();
    
    if parts.len() != 2 {
        return EmailValidation {
            valid: false,
            reason: "Invalid email format (no @)".to_string(),
        };
    }
    
    let domain = parts[1];
    
    if domain.len() < 3 {
        return EmailValidation {
            valid: false,
            reason: "Domain too short".to_string(),
        };
    }
    
    if !domain.chars().all(|c| c.is_alphanumeric() || c == '.' || c == '-') {
        return EmailValidation {
            valid: false,
            reason: "Invalid domain characters".to_string(),
        };
    }
    
    // Check MX record
    let output = std::process::Command::new("nslookup")
        .args(&["-type=mx", domain])
        .output();
    
    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            
            if stdout.contains("mail exchanger") || stdout.contains("MX preference") {
                EmailValidation {
                    valid: true,
                    reason: "MX records found".to_string(),
                }
            } else {
                EmailValidation {
                    valid: false,
                    reason: "No MX records found".to_string(),
                }
            }
        }
        Err(e) => EmailValidation {
            valid: false,
            reason: format!("DNS lookup failed: {}", e),
        },
    }
}

pub struct EmailValidation {
    pub valid: bool,
    pub reason: String,
}

impl std::fmt::Display for EmailValidation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.valid {
            write!(f, "✓ VALID: {}", self.reason)
        } else {
            write!(f, "✗ INVALID: {}", self.reason)
        }
    }
}
