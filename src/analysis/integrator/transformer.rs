// analysis/integrator/transformer.rs - Data transformer

use super::merger::{DataMerger, IntegrationResult};

pub fn integrate_data(sources: Vec<std::collections::HashMap<String, String>>) -> IntegrationResult {
    let mut merger = DataMerger::new();
    
    for source in sources {
        merger.add_source(source);
    }
    
    merger.merge()
}

pub fn transform_email_data(email_data: &str) -> Vec<(String, String)> {
    let mut result = Vec::new();
    
    for line in email_data.lines() {
        if let Some((key, value)) = line.split_once(':') {
            result.push((key.trim().to_string(), value.trim().to_string()));
        }
    }
    
    result
}

pub fn validate_email_list(emails: &[String]) -> Vec<(String, bool)> {
    emails.iter().map(|email| {
        let valid = email.contains('@') && email.contains('.') && !email.contains(' ');
        (email.clone(), valid)
    }).collect()
}
