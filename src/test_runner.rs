use std::time::Duration;
use std::sync::Arc;
use tokio::sync::Semaphore;

pub struct TestRunner {
    max_concurrent: usize,
    semaphore: Arc<Semaphore>,
}

impl TestRunner {
    pub fn new() -> Self {
        Self {
            max_concurrent: 4,
            semaphore: Arc::new(Semaphore::new(4)),
        }
    }

    pub fn max_concurrent(mut self, n: usize) -> Self {
        self.max_concurrent = n;
        self.semaphore = Arc::new(Semaphore::new(n));
        self
    }

    pub fn build(self) -> Self {
        self
    }

    pub fn get_max_concurrent(&self) -> usize {
        self.max_concurrent
    }
}

#[derive(Debug, Clone)]
pub struct TestDescriptor {
    pub name: String,
    pub tags: Vec<String>,
    pub timeout: Duration,
    pub retries: u32,
}

impl Default for TestDescriptor {
    fn default() -> Self {
        Self {
            name: String::new(),
            tags: Vec::new(),
            timeout: Duration::from_secs(30),
            retries: 3,
        }
    }
}

#[derive(Debug)]
pub enum TestResult {
    Passed { duration: Duration },
    Failed { duration: Duration, error: String },
    Skipped { reason: String },
}
