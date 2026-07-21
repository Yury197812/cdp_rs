// orchestrator/cache/lru.rs - LRU Cache
use std::collections::HashMap;

pub struct LruCache<K, V> {
    capacity: usize,
    entries: Vec<(K, V)>,
}

impl<K: Clone + Eq + std::hash::Hash, V: Clone> LruCache<K, V> {
    pub fn new(capacity: usize) -> Self {
        LruCache {
            capacity,
            entries: Vec::new(),
        }
    }
    
    pub fn get(&mut self, key: &K) -> Option<&V> {
        if let Some(pos) = self.entries.iter().position(|(k, _)| k == key) {
            let entry = self.entries.remove(pos);
            self.entries.push(entry);
            self.entries.last().map(|(_, v)| v)
        } else {
            None
        }
    }
    
    pub fn insert(&mut self, key: K, value: V) {
        if let Some(pos) = self.entries.iter().position(|(k, _)| *k == key) {
            self.entries.remove(pos);
        }
        
        if self.entries.len() >= self.capacity {
            self.entries.remove(0);
        }
        
        self.entries.push((key, value));
    }
    
    pub fn remove(&mut self, key: &K) -> Option<V> {
        if let Some(pos) = self.entries.iter().position(|(k, _)| k == key) {
            Some(self.entries.remove(pos).1)
        } else {
            None
        }
    }
    
    pub fn contains(&self, key: &K) -> bool {
        self.entries.iter().any(|(k, _)| k == key)
    }
    
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}
