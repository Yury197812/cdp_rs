use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;

use crate::browser::CdpConnection;

/// Test runner with parallel execution, retries, and reporting
pub struct TestRunner {
    max_concurrent: usize,
    semaphore: Arc<Semaphore>,
    results: Vec<TestRun>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestRun {
    pub name: String,
    pub status: TestStatus,
    pub duration_ms: u64,
    pub attempt: u32,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TestStatus {
    Passed,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestReport {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub total_duration_ms: u64,
    pub tests: Vec<TestRun>,
}

impl TestRunner {
    pub fn new() -> Self {
        Self {
            max_concurrent: 4,
            semaphore: Arc::new(Semaphore::new(4)),
            results: Vec::new(),
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

    /// Run a single test with retries
    pub async fn run_test<F, Fut>(
        &self,
        descriptor: &TestDescriptor,
        test_fn: F,
    ) -> TestRun
    where
        F: Fn(Arc<CdpConnection>) -> Fut,
        Fut: std::future::Future<Output = Result<()>>,
    {
        let _permit = self.semaphore.acquire().await.unwrap();
        let start = Instant::now();
        let mut last_error = None;

        for attempt in 1..=descriptor.retries {
            // Launch a fresh browser for each attempt
            let browser = match crate::browser::BrowserManager::new()
                .binary(r"C:\Program Files\Google\Chrome\Application\chrome.exe")
                .launch()
                .await
            {
                Ok(b) => b,
                Err(e) => {
                    last_error = Some(format!("Launch failed: {}", e));
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    continue;
                }
            };

            let conn = browser.connection().unwrap();

            match tokio::time::timeout(descriptor.timeout, test_fn(conn.clone())).await {
                Ok(Ok(())) => {
                    return TestRun {
                        name: descriptor.name.clone(),
                        status: TestStatus::Passed,
                        duration_ms: start.elapsed().as_millis() as u64,
                        attempt,
                        error: None,
                    };
                }
                Ok(Err(e)) => {
                    last_error = Some(format!("{}", e));
                }
                Err(_) => {
                    last_error = Some(format!("Timeout after {:?}", descriptor.timeout));
                }
            }
        }

        TestRun {
            name: descriptor.name.clone(),
            status: TestStatus::Failed,
            duration_ms: start.elapsed().as_millis() as u64,
            attempt: descriptor.retries,
            error: last_error,
        }
    }

    /// Run multiple tests in parallel
    pub async fn run_suite<F, Fut>(
        &mut self,
        tests: Vec<(TestDescriptor, F)>,
    ) -> TestReport
    where
        F: Fn(Arc<CdpConnection>) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<()>> + Send + 'static,
    {
        let start = Instant::now();
        let mut handles = Vec::new();
        let runner = Arc::new(self.clone());

        for (desc, test_fn) in tests {
            let runner = runner.clone();
            let desc = desc.clone();
            let handle = tokio::spawn(async move {
                runner.run_test(&desc, test_fn).await
            });
            handles.push(handle);
        }

        let mut results = Vec::new();
        for handle in handles {
            if let Ok(run) = handle.await {
                results.push(run);
            }
        }

        let total = results.len();
        let passed = results.iter().filter(|r| r.status == TestStatus::Passed).count();
        let failed = results.iter().filter(|r| r.status == TestStatus::Failed).count();
        let skipped = results.iter().filter(|r| r.status == TestStatus::Skipped).count();

        self.results = results.clone();

        TestReport {
            total,
            passed,
            failed,
            skipped,
            total_duration_ms: start.elapsed().as_millis() as u64,
            tests: results,
        }
    }

    /// Generate HTML report
    pub fn to_html(&self) -> String {
        let passed = self.results.iter().filter(|r| r.status == TestStatus::Passed).count();
        let failed = self.results.iter().filter(|r| r.status == TestStatus::Failed).count();
        let total = self.results.len();

        let rows: String = self.results.iter().map(|r| {
            let status_class = match r.status {
                TestStatus::Passed => "passed",
                TestStatus::Failed => "failed",
                TestStatus::Skipped => "skipped",
            };
            let error_cell = match &r.error {
                Some(e) => format!("<td class='error'>{}</td>", e),
                None => "<td></td>".to_string(),
            };
            format!(
                "<tr class='{}'><td>{}</td><td>{}ms</td><td>{}</td>{}</tr>",
                status_class, r.name, r.duration_ms, r.attempt, error_cell
            )
        }).collect();

        format!(
            r#"<!DOCTYPE html>
<html><head><title>CDP-RS Test Report</title>
<style>
body {{ font-family: monospace; margin: 20px; }}
table {{ border-collapse: collapse; width: 100%; }}
th, td {{ border: 1px solid #333; padding: 8px; text-align: left; }}
.passed {{ background: #1a472a; }}
.failed {{ background: #7a1a1a; }}
.skipped {{ background: #4a4a1a; }}
.error {{ color: #ff6b6b; font-size: 12px; }}
.summary {{ margin: 20px 0; font-size: 18px; }}
</style></head><body>
<h1>CDP-RS Test Report</h1>
<div class='summary'>{} passed, {} failed, {} total</div>
<table><tr><th>Test</th><th>Duration</th><th>Attempts</th><th>Error</th></tr>
{}</table>
</body></html>"#,
            passed, failed, total, rows
        )
    }
}

impl Clone for TestRunner {
    fn clone(&self) -> Self {
        Self {
            max_concurrent: self.max_concurrent,
            semaphore: Arc::new(Semaphore::new(self.max_concurrent)),
            results: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runner_new() {
        let runner = TestRunner::new();
        assert_eq!(runner.get_max_concurrent(), 4);
    }

    #[test]
    fn test_runner_builder() {
        let runner = TestRunner::new()
            .max_concurrent(8)
            .build();
        assert_eq!(runner.get_max_concurrent(), 8);
    }

    #[test]
    fn test_descriptor_default() {
        let desc = TestDescriptor::default();
        assert!(desc.name.is_empty());
        assert!(desc.tags.is_empty());
        assert_eq!(desc.timeout, Duration::from_secs(30));
        assert_eq!(desc.retries, 3);
    }

    #[test]
    fn test_report_empty() {
        let runner = TestRunner::new();
        let html = runner.to_html();
        assert!(html.contains("0 passed"));
        assert!(html.contains("0 failed"));
    }

    #[test]
    fn test_test_run_clone() {
        let run = TestRun {
            name: "test1".to_string(),
            status: TestStatus::Passed,
            duration_ms: 100,
            attempt: 1,
            error: None,
        };
        let cloned = run.clone();
        assert_eq!(cloned.name, "test1");
        assert_eq!(cloned.status, TestStatus::Passed);
    }
}
