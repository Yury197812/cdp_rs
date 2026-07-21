// orchestrator/logging/metrics.rs - Metrics collection
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

pub struct Metrics {
    requests: Arc<Mutex<HashMap<String, u64>>>,
    errors: Arc<Mutex<HashMap<String, u64>>>,
    latencies: Arc<Mutex<Vec<f64>>>,
    start_time: Instant,
}

impl Metrics {
    pub fn new() -> Self {
        Metrics {
            requests: Arc::new(Mutex::new(HashMap::new())),
            errors: Arc::new(Mutex::new(HashMap::new())),
            latencies: Arc::new(Mutex::new(Vec::new())),
            start_time: Instant::now(),
        }
    }
    
    pub fn record_request(&self, endpoint: &str) {
        if let Ok(mut requests) = self.requests.lock() {
            *requests.entry(endpoint.to_string()).or_insert(0) += 1;
        }
    }
    
    pub fn record_error(&self, endpoint: &str) {
        if let Ok(mut errors) = self.errors.lock() {
            *errors.entry(endpoint.to_string()).or_insert(0) += 1;
        }
    }
    
    pub fn record_latency(&self, latency_ms: f64) {
        if let Ok(mut latencies) = self.latencies.lock() {
            latencies.push(latency_ms);
        }
    }
    
    pub fn get_stats(&self) -> MetricsStats {
        let requests = self.requests.lock().unwrap().clone();
        let errors = self.errors.lock().unwrap().clone();
        let latencies = self.latencies.lock().unwrap().clone();
        
        let total_requests: u64 = requests.values().sum();
        let total_errors: u64 = errors.values().sum();
        let avg_latency = if latencies.is_empty() {
            0.0
        } else {
            latencies.iter().sum::<f64>() / latencies.len() as f64
        };
        let uptime = self.start_time.elapsed().as_secs();
        
        MetricsStats {
            total_requests,
            total_errors,
            avg_latency_ms: avg_latency,
            uptime_secs: uptime,
            requests_by_endpoint: requests,
        }
    }
}

#[derive(Debug, serde::Serialize)]
pub struct MetricsStats {
    pub total_requests: u64,
    pub total_errors: u64,
    pub avg_latency_ms: f64,
    pub uptime_secs: u64,
    pub requests_by_endpoint: HashMap<String, u64>,
}
