// orchestrator/coordinator/data_exchange.rs - Data exchange between branches
use std::collections::HashMap;

pub struct DataExchange {
    shared_data: HashMap<String, String>,
}

impl DataExchange {
    pub fn new() -> Self {
        DataExchange {
            shared_data: HashMap::new(),
        }
    }
    
    /// Store data with key
    pub fn store(&mut self, key: &str, value: &str) {
        self.shared_data.insert(key.to_string(), value.to_string());
    }
    
    /// Retrieve data by key
    pub fn retrieve(&self, key: &str) -> Option<&String> {
        self.shared_data.get(key)
    }
    
    /// Check if key exists
    pub fn has(&self, key: &str) -> bool {
        self.shared_data.contains_key(key)
    }
    
    /// Remove data by key
    pub fn remove(&mut self, key: &str) -> Option<String> {
        self.shared_data.remove(key)
    }
    
    /// Get all keys
    pub fn keys(&self) -> Vec<&String> {
        self.shared_data.keys().collect()
    }
    
    /// Get all data
    pub fn all(&self) -> &HashMap<String, String> {
        &self.shared_data
    }
    
    /// Clear all data
    pub fn clear(&mut self) {
        self.shared_data.clear();
    }
    
    /// Export data as JSON
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(&self.shared_data).unwrap_or_default()
    }
    
    /// Import data from JSON
    pub fn from_json(&mut self, json: &str) -> Result<(), String> {
        if let Ok(data) = serde_json::from_str::<HashMap<String, String>>(json) {
            self.shared_data.extend(data);
            Ok(())
        } else {
            Err("Invalid JSON".to_string())
        }
    }
}
