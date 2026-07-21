// database/pool/manager.rs - Connection pool manager

use std::sync::{Arc, Mutex};
use std::collections::VecDeque;

pub struct ConnectionPool {
    connections: VecDeque<Arc<Mutex<()>>>,
    max_size: usize,
}

impl ConnectionPool {
    pub fn new(max_size: usize) -> Self {
        let mut connections = VecDeque::new();
        for _ in 0..max_size {
            connections.push_back(Arc::new(Mutex::new(())));
        }
        
        ConnectionPool {
            connections,
            max_size,
        }
    }
    
    pub fn acquire(&mut self) -> Option<Arc<Mutex<()>>> {
        self.connections.pop_front()
    }
    
    pub fn release(&mut self, conn: Arc<Mutex<()>>) {
        self.connections.push_back(conn);
    }
    
    pub fn available(&self) -> usize {
        self.connections.len()
    }
    
    pub fn total(&self) -> usize {
        self.max_size
    }
}
