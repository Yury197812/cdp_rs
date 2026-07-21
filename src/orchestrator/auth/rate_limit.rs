// orchestrator/auth/rate_limit.rs - Rate limiter
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub struct RateLimiter {
    limits: HashMap<String, RateLimit>,
    window: Duration,
}

struct RateLimit {
    count: u32,
    last_reset: Instant,
}

impl RateLimiter {
    pub fn new(window_secs: u64) -> Self {
        RateLimiter {
            limits: HashMap::new(),
            window: Duration::from_secs(window_secs),
        }
    }
    
    pub fn check(&mut self, key: &str, max_requests: u32) -> bool {
        let now = Instant::now();
        
        let limit = self.limits.entry(key.to_string()).or_insert(RateLimit {
            count: 0,
            last_reset: now,
        });
        
        if now.duration_since(limit.last_reset) > self.window {
            limit.count = 0;
            limit.last_reset = now;
        }
        
        if limit.count >= max_requests {
            return false;
        }
        
        limit.count += 1;
        true
    }
    
    pub fn remaining(&self, key: &str, max_requests: u32) -> u32 {
        match self.limits.get(key) {
            Some(limit) => {
                let now = Instant::now();
                if now.duration_since(limit.last_reset) > self.window {
                    max_requests
                } else {
                    max_requests.saturating_sub(limit.count)
                }
            }
            None => max_requests,
        }
    }
}

pub struct RateLimitConfig {
    pub window_secs: u64,
    pub max_requests: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        RateLimitConfig {
            window_secs: 60,
            max_requests: 100,
        }
    }
}
