use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::browser::{CdpConnection, CdpEvent};

/// Network request/response interceptor with recording and modification
pub struct NetworkInterceptor {
    connection: Arc<CdpConnection>,
    rules: Arc<RwLock<Vec<InterceptRule>>>,
    log: Arc<RwLock<Vec<NetworkEntry>>>,
    options: InterceptOptions,
}

#[derive(Clone, Debug)]
pub struct InterceptOptions {
    pub record_request_body: bool,
    pub record_response_body: bool,
    pub max_body_size: usize,
    pub filter_urls: Vec<String>,
    pub ignore_urls: Vec<String>,
}

impl Default for InterceptOptions {
    fn default() -> Self {
        Self {
            record_request_body: true,
            record_response_body: true,
            max_body_size: 1024 * 1024, // 1MB
            filter_urls: Vec::new(),
            ignore_urls: vec![
                "data:".to_string(),
                "chrome-extension:".to_string(),
            ],
        }
    }
}

impl InterceptOptions {
    pub fn new() -> Self { Self::default() }

    pub fn record_request_body(mut self, v: bool) -> Self {
        self.record_request_body = v; self
    }

    pub fn record_response_body(mut self, v: bool) -> Self {
        self.record_response_body = v; self
    }

    pub fn max_body_size(mut self, bytes: usize) -> Self {
        self.max_body_size = bytes; self
    }

    pub fn filter_urls(mut self, urls: Vec<String>) -> Self {
        self.filter_urls = urls; self
    }

    pub fn ignore_urls(mut self, urls: Vec<String>) -> Self {
        self.ignore_urls = urls; self
    }
}

/// Intercept rule for modifying requests/responses
#[derive(Clone)]
pub struct InterceptRule {
    pub pattern: String,
    pub action: InterceptAction,
}

#[derive(Clone)]
pub enum InterceptAction {
    Block,
    Mock { status: u16, body: String, content_type: String },
    ModifyRequest { headers: Option<HashMap<String, String>>, body: Option<String> },
    ModifyResponse { status: Option<u16>, headers: Option<HashMap<String, String>>, body: Option<String> },
    Log,
}

/// Recorded network entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkEntry {
    pub id: String,
    pub timestamp: f64,
    pub request: RequestInfo,
    pub response: Option<ResponseInfo>,
    pub timings: Option<TimingsInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestInfo {
    pub url: String,
    pub method: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    pub resource_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseInfo {
    pub status: u16,
    pub status_text: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    pub content_type: Option<String>,
    pub content_length: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimingsInfo {
    pub dns: Option<f64>,
    pub connect: Option<f64>,
    pub ssl: Option<f64>,
    pub send: Option<f64>,
    pub wait: Option<f64>,
    pub receive: Option<f64>,
}

/// HAR export format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarLog {
    pub log: HarLogInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarLogInfo {
    pub version: String,
    pub creator: HarCreator,
    pub entries: Vec<HarEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarCreator {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarEntry {
    pub started_iso_date: String,
    pub time: f64,
    pub request: HarRequest,
    pub response: HarResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarRequest {
    pub method: String,
    pub url: String,
    pub headers: Vec<HarNameValuePair>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarResponse {
    pub status: u16,
    pub status_text: String,
    pub headers: Vec<HarNameValuePair>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarNameValuePair {
    pub name: String,
    pub value: String,
}

impl NetworkInterceptor {
    pub fn new(connection: Arc<CdpConnection>) -> Self {
        Self::with_options(connection, InterceptOptions::default())
    }

    pub fn with_options(connection: Arc<CdpConnection>, options: InterceptOptions) -> Self {
        Self {
            connection,
            rules: Arc::new(RwLock::new(Vec::new())),
            log: Arc::new(RwLock::new(Vec::new())),
            options,
        }
    }

    /// Enable network interception
    pub async fn enable(&self) -> Result<()> {
        self.connection.send_page("Network.enable", json!({
            "maxTotalBufferSize": 10485760,
            "maxResourceBufferSize": 1048576,
        })).await?;
        Ok(())
    }

    /// Disable network interception
    pub async fn disable(&self) -> Result<()> {
        self.connection.send_page("Network.disable", json!({})).await?;
        Ok(())
    }

    /// Enable request interception via Fetch domain
    pub async fn enable_fetch(&self, patterns: &[&str]) -> Result<()> {
        let patterns: Vec<Value> = patterns.iter()
            .map(|p| json!({"urlPattern": p, "requestStage": "Request"}))
            .collect();
        self.connection.send_page("Fetch.enable", json!({
            "patterns": patterns,
        })).await?;
        Ok(())
    }

    /// Disable request interception
    pub async fn disable_fetch(&self) -> Result<()> {
        self.connection.send_page("Fetch.disable", json!({})).await?;
        Ok(())
    }

    /// Add an intercept rule
    pub async fn add_rule(&self, rule: InterceptRule) {
        self.rules.write().await.push(rule);
    }

    /// Block URLs matching pattern
    pub async fn block(&self, pattern: &str) {
        self.add_rule(InterceptRule {
            pattern: pattern.to_string(),
            action: InterceptAction::Block,
        }).await;
    }

    /// Mock URL with response
    pub async fn mock(&self, pattern: &str, status: u16, body: &str, content_type: &str) {
        self.add_rule(InterceptRule {
            pattern: pattern.to_string(),
            action: InterceptAction::Mock {
                status,
                body: body.to_string(),
                content_type: content_type.to_string(),
            },
        }).await;
    }

    /// Modify request headers
    pub async fn modify_request_headers(&self, pattern: &str, headers: HashMap<String, String>) {
        self.add_rule(InterceptRule {
            pattern: pattern.to_string(),
            action: InterceptAction::ModifyRequest {
                headers: Some(headers),
                body: None,
            },
        }).await;
    }

    /// Modify response
    pub async fn modify_response(&self, pattern: &str, status: Option<u16>, body: Option<String>) {
        self.add_rule(InterceptRule {
            pattern: pattern.to_string(),
            action: InterceptAction::ModifyResponse {
                status,
                headers: None,
                body,
            },
        }).await;
    }

    /// Log all requests (pass-through)
    pub async fn log_all(&self) {
        self.add_rule(InterceptRule {
            pattern: "*".to_string(),
            action: InterceptAction::Log,
        }).await;
    }

    /// Block common ad/tracking domains
    pub async fn block_ads(&self) {
        let patterns = [
            "doubleclick.net", "googlesyndication.com", "google-analytics.com",
            "adservice.google.com", "facebook.com/tr", "hotjar.com", "mixpanel.com",
            "segment.com", "amplitude.com", "branch.io", "adjust.com",
        ];
        for p in &patterns { self.block(p).await; }
    }

    /// Process a network event — call this in your event loop
    pub async fn handle_event(&self, event: &CdpEvent) -> Result<()> {
        match event.method.as_str() {
            "Network.requestWillBeSent" => {
                let req_id = event.params["requestId"].as_str().unwrap_or("");
                let url = event.params["request"]["url"].as_str().unwrap_or("");
                let method = event.params["request"]["method"].as_str().unwrap_or("GET");
                let resource_type = event.params["type"].as_str().unwrap_or("Other");

                if self.should_ignore(url) { return Ok(()); }

                let mut headers = HashMap::new();
                if let Some(h) = event.params["request"]["headers"].as_object() {
                    for (k, v) in h {
                        headers.insert(k.clone(), v.as_str().unwrap_or("").to_string());
                    }
                }

                let entry = NetworkEntry {
                    id: req_id.to_string(),
                    timestamp: event.params["timestamp"].as_f64().unwrap_or(0.0),
                    request: RequestInfo {
                        url: url.to_string(),
                        method: method.to_string(),
                        headers,
                        body: None,
                        resource_type: resource_type.to_string(),
                    },
                    response: None,
                    timings: None,
                };

                self.log.write().await.push(entry);
            }
            "Network.responseReceived" => {
                let req_id = event.params["requestId"].as_str().unwrap_or("");
                let log = self.log.read().await;
                if let Some(entry) = log.iter().find(|e| e.id == req_id) {
                    let mut entry = entry.clone();
                    let resp = &event.params["response"];
                    let mut resp_headers = HashMap::new();
                    if let Some(h) = resp["headers"].as_object() {
                        for (k, v) in h {
                            resp_headers.insert(k.clone(), v.as_str().unwrap_or("").to_string());
                        }
                    }
                    let content_type = resp_headers.get("content-type").cloned();
                    let content_length = resp_headers.get("content-length")
                        .and_then(|v| v.parse::<u64>().ok());

                    entry.response = Some(ResponseInfo {
                        status: resp["status"].as_u64().unwrap_or(0) as u16,
                        status_text: resp["statusText"].as_str().unwrap_or("").to_string(),
                        headers: resp_headers,
                        body: None,
                        content_type,
                        content_length,
                    });

                    drop(log);
                    let mut log = self.log.write().await;
                    if let Some(e) = log.iter_mut().find(|e| e.id == req_id) {
                        *e = entry;
                    }
                }
            }
            "Fetch.requestPaused" => {
                let req_id = event.params["requestId"].as_str().unwrap_or("");
                let url = event.params["request"]["url"].as_str().unwrap_or("");

                let rules = self.rules.read().await;
                for rule in rules.iter() {
                    if url.contains(&rule.pattern) || rule.pattern == "*" {
                        match &rule.action {
                            InterceptAction::Block => {
                                self.connection.send_page("Fetch.failRequest", json!({
                                    "requestId": req_id,
                                    "reason": "BlockedByClient",
                                })).await?;
                                return Ok(());
                            }
                            InterceptAction::Mock { status, body, content_type } => {
                                use base64::Engine;
                                let encoded = base64::engine::general_purpose::STANDARD.encode(body.as_bytes());
                                self.connection.send_page("Fetch.fulfillRequest", json!({
                                    "requestId": req_id,
                                    "responseCode": status,
                                    "responseHeaders": [
                                        {"name": "Content-Type", "value": content_type},
                                    ],
                                    "body": encoded,
                                })).await?;
                                return Ok(());
                            }
                            _ => {}
                        }
                    }
                }
                drop(rules);

                // Continue the request
                self.connection.send_page("Fetch.continueRequest", json!({
                    "requestId": req_id,
                })).await?;
            }
            "Network.loadingFinished" => {
                // Request completed — body could be fetched here if needed
            }
            _ => {}
        }
        Ok(())
    }

    /// Get all recorded entries
    pub async fn entries(&self) -> Vec<NetworkEntry> {
        self.log.read().await.clone()
    }

    /// Get entries matching a URL pattern
    pub async fn entries_for(&self, pattern: &str) -> Vec<NetworkEntry> {
        self.log.read().await.iter()
            .filter(|e| e.request.url.contains(pattern))
            .cloned()
            .collect()
    }

    /// Get entries by method
    pub async fn entries_by_method(&self, method: &str) -> Vec<NetworkEntry> {
        self.log.read().await.iter()
            .filter(|e| e.request.method.eq_ignore_ascii_case(method))
            .cloned()
            .collect()
    }

    /// Get entries by status code
    pub async fn entries_by_status(&self, status: u16) -> Vec<NetworkEntry> {
        self.log.read().await.iter()
            .filter(|e| e.response.as_ref().map(|r| r.status == status).unwrap_or(false))
            .cloned()
            .collect()
    }

    /// Get failed requests (4xx, 5xx)
    pub async fn failed_entries(&self) -> Vec<NetworkEntry> {
        self.log.read().await.iter()
            .filter(|e| e.response.as_ref()
                .map(|r| r.status >= 400)
                .unwrap_or(false))
            .cloned()
            .collect()
    }

    /// Get total request count
    pub async fn request_count(&self) -> usize {
        self.log.read().await.len()
    }

    /// Get total transferred bytes
    pub async fn total_bytes(&self) -> u64 {
        self.log.read().await.iter()
            .filter_map(|e| e.response.as_ref())
            .filter_map(|r| r.content_length)
            .sum()
    }

    /// Clear recorded entries
    pub async fn clear_log(&self) {
        self.log.write().await.clear();
    }

    /// Export to HAR format
    pub async fn to_har(&self) -> HarLog {
        let entries = self.log.read().await;
        let har_entries: Vec<HarEntry> = entries.iter().map(|e| {
            HarEntry {
                started_iso_date: format!("{:.3}", e.timestamp),
                time: e.timings.as_ref()
                    .map(|t| t.receive.unwrap_or(0.0) + t.wait.unwrap_or(0.0))
                    .unwrap_or(0.0),
                request: HarRequest {
                    method: e.request.method.clone(),
                    url: e.request.url.clone(),
                    headers: e.request.headers.iter()
                        .map(|(k, v)| HarNameValuePair { name: k.clone(), value: v.clone() })
                        .collect(),
                },
                response: HarResponse {
                    status: e.response.as_ref().map(|r| r.status).unwrap_or(0),
                    status_text: e.response.as_ref().map(|r| r.status_text.clone()).unwrap_or_default(),
                    headers: e.response.as_ref()
                        .map(|r| r.headers.iter()
                            .map(|(k, v)| HarNameValuePair { name: k.clone(), value: v.clone() })
                            .collect())
                        .unwrap_or_default(),
                },
            }
        }).collect();

        HarLog {
            log: HarLogInfo {
                version: "1.2".to_string(),
                creator: HarCreator {
                    name: "cdp_rs".to_string(),
                    version: "0.2.0".to_string(),
                },
                entries: har_entries,
            },
        }
    }

    /// Save HAR log to file
    pub async fn save_har(&self, path: &str) -> Result<()> {
        let har = self.to_har().await;
        let json = serde_json::to_string_pretty(&har)?;
        tokio::fs::write(path, json).await?;
        Ok(())
    }

    /// Export as simple text log
    pub async fn to_text_log(&self) -> String {
        let entries = self.log.read().await;
        entries.iter().map(|e| {
            let status = e.response.as_ref().map(|r| r.status).unwrap_or(0);
            let size = e.response.as_ref()
                .and_then(|r| r.content_length)
                .map(|l| format!("{}b", l))
                .unwrap_or_else(|| "-".to_string());
            format!("[{}] {} {} {} ({})", e.id, e.request.method, status, e.request.url, size)
        }).collect::<Vec<_>>().join("\n")
    }

    fn should_ignore(&self, url: &str) -> bool {
        self.options.ignore_urls.iter().any(|p| url.contains(p.as_str()))
    }
}

/// Helper to create intercept rules
pub fn block(pattern: &str) -> InterceptRule {
    InterceptRule { pattern: pattern.to_string(), action: InterceptAction::Block }
}

pub fn mock(pattern: &str, status: u16, body: &str) -> InterceptRule {
    InterceptRule {
        pattern: pattern.to_string(),
        action: InterceptAction::Mock {
            status,
            body: body.to_string(),
            content_type: "text/plain".to_string(),
        },
    }
}

pub fn mock_json(pattern: &str, status: u16, body: &Value) -> InterceptRule {
    InterceptRule {
        pattern: pattern.to_string(),
        action: InterceptAction::Mock {
            status,
            body: body.to_string(),
            content_type: "application/json".to_string(),
        },
    }
}

pub fn modify_headers(pattern: &str, headers: HashMap<String, String>) -> InterceptRule {
    InterceptRule {
        pattern: pattern.to_string(),
        action: InterceptAction::ModifyRequest {
            headers: Some(headers),
            body: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intercept_options_default() {
        let opts = InterceptOptions::new();
        assert!(opts.record_request_body);
        assert!(opts.record_response_body);
        assert_eq!(opts.max_body_size, 1024 * 1024);
    }

    #[test]
    fn test_intercept_options_builder() {
        let opts = InterceptOptions::new()
            .record_request_body(false)
            .record_response_body(false)
            .max_body_size(512)
            .ignore_urls(vec!["test:".to_string()]);
        assert!(!opts.record_request_body);
        assert_eq!(opts.max_body_size, 512);
    }

    #[test]
    fn test_network_entry_serialize() {
        let entry = NetworkEntry {
            id: "1".to_string(),
            timestamp: 1234567890.0,
            request: RequestInfo {
                url: "https://example.com".to_string(),
                method: "GET".to_string(),
                headers: HashMap::new(),
                body: None,
                resource_type: "Document".to_string(),
            },
            response: Some(ResponseInfo {
                status: 200,
                status_text: "OK".to_string(),
                headers: HashMap::new(),
                body: None,
                content_type: Some("text/html".to_string()),
                content_length: Some(1234),
            }),
            timings: None,
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("example.com"));
        assert!(json.contains("200"));
    }

    #[test]
    fn test_har_log_serialize() {
        let har = HarLog {
            log: HarLogInfo {
                version: "1.2".to_string(),
                creator: HarCreator {
                    name: "test".to_string(),
                    version: "1.0".to_string(),
                },
                entries: vec![],
            },
        };
        let json = serde_json::to_string(&har).unwrap();
        assert!(json.contains("1.2"));
        assert!(json.contains("test"));
    }

    #[test]
    fn test_intercept_rule_block() {
        let rule = block("ads.example.com");
        assert_eq!(rule.pattern, "ads.example.com");
        match rule.action {
            InterceptAction::Block => {},
            _ => panic!("Expected Block"),
        }
    }

    #[test]
    fn test_intercept_rule_mock() {
        let rule = mock("api.test.com", 200, "ok");
        assert_eq!(rule.pattern, "api.test.com");
        match &rule.action {
            InterceptAction::Mock { status, body, .. } => {
                assert_eq!(*status, 200);
                assert_eq!(body, "ok");
            }
            _ => panic!("Expected Mock"),
        }
    }

    #[test]
    fn test_intercept_rule_mock_json() {
        let rule = mock_json("api.test.com", 201, &json!({"created": true}));
        assert_eq!(rule.pattern, "api.test.com");
    }

    #[test]
    fn test_intercept_rule_modify_headers() {
        let mut headers = HashMap::new();
        headers.insert("Authorization".to_string(), "Bearer tok".to_string());
        let rule = modify_headers("api.test.com", headers);
        match &rule.action {
            InterceptAction::ModifyRequest { headers: Some(h), .. } => {
                assert_eq!(h.get("Authorization").unwrap(), "Bearer tok");
            }
            _ => panic!("Expected ModifyRequest with headers"),
        }
    }

    #[test]
    fn test_har_entry_serialize() {
        let entry = HarEntry {
            started_iso_date: "2024-01-01".to_string(),
            time: 100.0,
            request: HarRequest {
                method: "GET".to_string(),
                url: "https://example.com".to_string(),
                headers: vec![],
            },
            response: HarResponse {
                status: 200,
                status_text: "OK".to_string(),
                headers: vec![],
            },
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("GET"));
        assert!(json.contains("200"));
    }

    #[test]
    fn test_network_entry_clone() {
        let entry = NetworkEntry {
            id: "1".to_string(),
            timestamp: 0.0,
            request: RequestInfo {
                url: "test".to_string(),
                method: "GET".to_string(),
                headers: HashMap::new(),
                body: None,
                resource_type: "Other".to_string(),
            },
            response: None,
            timings: None,
        };
        let cloned = entry.clone();
        assert_eq!(cloned.id, "1");
    }
}
