// orchestrator/coordinator/sync_manager.rs - Synchronization manager
use std::collections::HashMap;

pub struct SyncManager {
    last_sync: HashMap<String, u64>,
}

impl SyncManager {
    pub fn new() -> Self {
        SyncManager {
            last_sync: HashMap::new(),
        }
    }
    
    /// Record sync timestamp
    pub fn record_sync(&mut self, key: &str) {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.last_sync.insert(key.to_string(), timestamp);
    }
    
    /// Check if sync is needed
    pub fn needs_sync(&self, key: &str, threshold_secs: u64) -> bool {
        match self.last_sync.get(key) {
            Some(last) => {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                now - last > threshold_secs
            }
            None => true,
        }
    }
    
    /// Get last sync time
    pub fn last_sync_time(&self, key: &str) -> Option<u64> {
        self.last_sync.get(key).copied()
    }
    
    /// Clear all sync records
    pub fn clear(&mut self) {
        self.last_sync.clear();
    }
}
