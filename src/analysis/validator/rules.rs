// analysis/validator/rules.rs - Validation rules

use super::types::ValidationResult;

pub fn validate_input(input: &str, rules: &[ValidationRule]) -> ValidationResult {
    let mut result = ValidationResult::new();
    
    for rule in rules {
        match rule {
            ValidationRule::NotEmpty => {
                if input.trim().is_empty() {
                    result.add_error("Input is empty".to_string());
                }
            }
            ValidationRule::MinLength(len) => {
                if input.len() < *len {
                    result.add_error(format!("Input too short (min {} chars)", len));
                }
            }
            ValidationRule::MaxLength(len) => {
                if input.len() > *len {
                    result.add_error(format!("Input too long (max {} chars)", len));
                }
            }
            ValidationRule::Contains(pattern) => {
                if !input.contains(pattern) {
                    result.add_warning(format!("Input doesn't contain '{}'", pattern));
                }
            }
            ValidationRule::EmailFormat => {
                if !input.contains('@') || !input.contains('.') {
                    result.add_error("Invalid email format".to_string());
                }
            }
        }
    }
    
    result
}

pub enum ValidationRule {
    NotEmpty,
    MinLength(usize),
    MaxLength(usize),
    Contains(String),
    EmailFormat,
}
