// orchestrator/cache/ttl.rs - TTL Cache
use std::collections::HashMap;
use std::time::{Duration, Instant};

pub struct TtlCache<K, V> {
    entries: HashMap<K, (V, Instant)>,
    ttl: Duration,
}

impl<K: Clone + Eq + std::hash::Hash, V: Clone> TtlCache<K, V> {
    pub fn new(ttl_secs: u64) -> Self {
        TtlCache {
            entries: HashMap::new(),
            ttl: Duration::from_secs(ttl_secs),
        }
    }
    
    pub fn get(&self, key: &K) -> Option<&V> {
        self.entries.get(key).and_then(|(v, instant)| {
            if instant.elapsed() < self.ttl {
                Some(v)
            } else {
                None
            }
        })
    }
    
    pub fn insert(&mut self, key: K, value: V) {
        self.entries.insert(key, (value, Instant::now()));
    }
    
    pub fn remove(&mut self, key: &K) -> Option<V> {
        self.entries.remove(key).map(|(v, _)| v)
    }
    
    pub fn cleanup(&mut self) {
        self.entries.retain(|_, (_, instant)| instant.elapsed() < self.ttl);
    }
    
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}
