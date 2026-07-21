// analysis/integrator/merger.rs - Data merger

use std::collections::HashMap;

pub struct IntegrationResult {
    pub success: bool,
    pub merged_data: HashMap<String, String>,
    pub conflicts: Vec<String>,
}

pub struct DataMerger {
    sources: Vec<HashMap<String, String>>,
}

impl DataMerger {
    pub fn new() -> Self {
        DataMerger {
            sources: Vec::new(),
        }
    }
    
    pub fn add_source(&mut self, data: HashMap<String, String>) {
        self.sources.push(data);
    }
    
    pub fn merge(&self) -> IntegrationResult {
        let mut merged = HashMap::new();
        let mut conflicts = Vec::new();
        
        for source in &self.sources {
            for (key, value) in source {
                if let Some(existing) = merged.get(key) {
                    if existing != value {
                        conflicts.push(format!("Conflict for key '{}': '{}' vs '{}'", key, existing, value));
                    }
                } else {
                    merged.insert(key.clone(), value.clone());
                }
            }
        }
        
        IntegrationResult {
            success: conflicts.is_empty(),
            merged_data: merged,
            conflicts,
        }
    }
}
