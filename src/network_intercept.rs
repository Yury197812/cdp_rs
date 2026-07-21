use anyhow::Result;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::browser::{CdpConnection, CdpEvent};

/// Intercepted request metadata
#[derive(Clone, Debug)]
pub struct InterceptedRequest {
    pub request_id: String,
    pub url: String,
    pub method: String,
    pub headers: Value,
    pub session_id: Option<String>,
}

/// Mock response for intercepted requests
#[derive(Clone, Debug)]
pub struct MockResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl MockResponse {
    pub fn json(status: u16, body: &Value) -> Self {
        Self {
            status,
            headers: vec![
                ("Content-Type".to_string(), "application/json".to_string()),
            ],
            body: body.to_string().into_bytes(),
        }
    }

    pub fn html(status: u16, body: &str) -> Self {
        Self {
            status,
            headers: vec![
                ("Content-Type".to_string(), "text/html; charset=utf-8".to_string()),
            ],
            body: body.as_bytes().to_vec(),
        }
    }

    pub fn text(status: u16, body: &str) -> Self {
        Self {
            status,
            headers: vec![
                ("Content-Type".to_string(), "text/plain; charset=utf-8".to_string()),
            ],
            body: body.as_bytes().to_vec(),
        }
    }
}

/// URL pattern matcher
#[derive(Clone)]
pub enum UrlPattern {
    Exact(String),
    Contains(String),
    Regex(String),
    Wildcard(String),
}

impl UrlPattern {
    pub fn matches(&self, url: &str) -> bool {
        match self {
            UrlPattern::Exact(s) => url == s,
            UrlPattern::Contains(s) => url.contains(s.as_str()),
            UrlPattern::Regex(r) => regex::Regex::new(r).map(|re| re.is_match(url)).unwrap_or(false),
            UrlPattern::Wildcard(w) => {
                let pattern = w.replace('*', ".*");
                regex::Regex::new(&pattern).map(|re| re.is_match(url)).unwrap_or(false)
            }
        }
    }
}

/// Intercept action for a matched request
#[derive(Clone)]
pub enum InterceptAction {
    /// Allow request to proceed
    Continue,
    /// Block request (return error)
    Block,
    /// Return mock response
    Mock(MockResponse),
    /// Modify headers before continuing
    ModifyHeaders(Vec<(String, String)>),
}

/// Network interceptor with pattern-based rules
pub struct NetworkInterceptor {
    connection: Arc<CdpConnection>,
    rules: Arc<RwLock<Vec<(UrlPattern, InterceptAction)>>>,
    paused_requests: Arc<RwLock<HashMap<String, InterceptedRequest>>>,
}

impl NetworkInterceptor {
    pub fn new(connection: Arc<CdpConnection>) -> Self {
        Self {
            connection,
            rules: Arc::new(RwLock::new(Vec::new())),
            paused_requests: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Enable Fetch domain for request interception
    pub async fn enable(&self) -> Result<()> {
        self.connection.send_page("Fetch.enable", json!({
            "patterns": [{"urlPattern": "*", "requestStage": "Request"}]
        })).await?;
        Ok(())
    }

    /// Disable Fetch domain
    pub async fn disable(&self) -> Result<()> {
        self.connection.send_page("Fetch.disable", json!({})).await?;
        Ok(())
    }

    /// Add interception rule
    pub async fn add_rule(&self, pattern: UrlPattern, action: InterceptAction) {
        self.rules.write().await.push((pattern, action));
    }

    /// Get number of active rules
    pub async fn rules_count(&self) -> usize {
        self.rules.read().await.len()
    }

    /// Block URLs matching pattern (shorthand)
    pub async fn block(&self, pattern: &str) {
        self.add_rule(
            UrlPattern::Contains(pattern.to_string()),
            InterceptAction::Block,
        ).await;
    }

    /// Mock URL with response (shorthand)
    pub async fn mock(&self, pattern: &str, response: MockResponse) {
        self.add_rule(
            UrlPattern::Contains(pattern.to_string()),
            InterceptAction::Mock(response),
        ).await;
    }

    /// Block common ad/tracking domains
    pub async fn block_ads(&self) {
        let ad_patterns = [
            "doubleclick.net",
            "googlesyndication.com",
            "google-analytics.com",
            "adservice.google.com",
            "facebook.com/tr",
            "hotjar.com",
            "mixpanel.com",
            "segment.com",
            "amplitude.com",
            "branch.io",
            "adjust.com",
        ];
        for pattern in &ad_patterns {
            self.block(pattern).await;
        }
    }

    /// Process intercepted events — call this in your event loop
    pub async fn handle_event(&self, event: &CdpEvent) -> Result<()> {
        if event.method != "Fetch.requestPaused" {
            return Ok(());
        }

        let request_id = event.params["requestId"].as_str().unwrap_or("");
        let url = event.params["request"]["url"].as_str().unwrap_or("");
        let method = event.params["request"]["method"].as_str().unwrap_or("GET");
        let headers = event.params["request"]["headers"].clone();
        let session_id = event.session_id.clone();

        // Store paused request
        let intercepted = InterceptedRequest {
            request_id: request_id.to_string(),
            url: url.to_string(),
            method: method.to_string(),
            headers: headers.clone(),
            session_id: session_id.clone(),
        };
        self.paused_requests.write().await.insert(request_id.to_string(), intercepted);

        // Check rules
        let rules = self.rules.read().await;
        let mut action = InterceptAction::Continue;
        for (pattern, act) in rules.iter() {
            if pattern.matches(url) {
                action = act.clone();
                break;
            }
        }
        drop(rules);

        // Execute action
        match action {
            InterceptAction::Continue => {
                self.continue_request(request_id, session_id.as_deref()).await?;
            }
            InterceptAction::Block => {
                self.fail_request(request_id, session_id.as_deref()).await?;
            }
            InterceptAction::Mock(response) => {
                self.fulfill_request(request_id, &response, session_id.as_deref()).await?;
            }
            InterceptAction::ModifyHeaders(new_headers) => {
                self.continue_with_headers(request_id, &new_headers, session_id.as_deref()).await?;
            }
        }

        self.paused_requests.write().await.remove(request_id);
        Ok(())
    }

    async fn continue_request(&self, request_id: &str, session_id: Option<&str>) -> Result<()> {
        self.send_fetch_command("Fetch.continueRequest", json!({
            "requestId": request_id
        }), session_id).await
    }

    async fn fail_request(&self, request_id: &str, session_id: Option<&str>) -> Result<()> {
        self.send_fetch_command("Fetch.failRequest", json!({
            "requestId": request_id,
            "reason": "BlockedByClient"
        }), session_id).await
    }

    async fn fulfill_request(&self, request_id: &str, response: &MockResponse, session_id: Option<&str>) -> Result<()> {
        use base64::Engine;
        let headers: Vec<Value> = response.headers.iter()
            .map(|(k, v)| json!({"name": k, "value": v}))
            .collect();

        self.send_fetch_command("Fetch.fulfillRequest", json!({
            "requestId": request_id,
            "responseCode": response.status,
            "responseHeaders": headers,
            "body": base64::engine::general_purpose::STANDARD.encode(&response.body),
        }), session_id).await
    }

    async fn continue_with_headers(&self, request_id: &str, headers: &[(String, String)], session_id: Option<&str>) -> Result<()> {
        let headers_val: Vec<Value> = headers.iter()
            .map(|(k, v)| json!({"name": k, "value": v}))
            .collect();

        self.send_fetch_command("Fetch.continueRequest", json!({
            "requestId": request_id,
            "headers": headers_val,
        }), session_id).await
    }

    async fn send_fetch_command(&self, method: &str, params: Value, session_id: Option<&str>) -> Result<()> {
        // If we have a session ID, we need to route through the session
        // For now, use direct connection (works for browser-level Fetch)
        self.connection.send_page(method, params).await?;
        Ok(())
    }
}

/// Predefined ad-block rules
pub struct AdBlocker;

impl AdBlocker {
    pub fn common_patterns() -> Vec<&'static str> {
        vec![
            "doubleclick.net",
            "googlesyndication.com",
            "google-analytics.com",
            "adservice.google.com",
            "pagead2.googlesyndication.com",
            "facebook.com/tr",
            "hotjar.com",
            "mixpanel.com",
            "segment.com",
            "amplitude.com",
            "branch.io",
            "adjust.com",
            "appsflyer.com",
            "singular.net",
            "kochava.com",
            "taboola.com",
            "outbrain.com",
            "criteo.com",
            "adskeeper.com",
            "propellerads.com",
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_url_pattern_matching() {
        assert!(UrlPattern::Contains("google".to_string()).matches("https://www.google.com"));
        assert!(UrlPattern::Exact("https://example.com".to_string()).matches("https://example.com"));
        assert!(!UrlPattern::Exact("https://example.com".to_string()).matches("https://other.com"));
    }

    #[tokio::test]
    async fn test_mock_response() {
        let resp = MockResponse::json(200, &json!({"ok": true}));
        assert_eq!(resp.status, 200);
        assert!(resp.headers.iter().any(|(k, _)| k == "Content-Type"));
    }

    #[test]
    fn test_url_pattern_contains_case() {
        let p = UrlPattern::Contains("ads".to_string());
        assert!(p.matches("https://ads.example.com"));
        assert!(!p.matches("https://example.com"));
    }

    #[test]
    fn test_url_pattern_exact_no_match() {
        let p = UrlPattern::Exact("https://a.com".to_string());
        assert!(!p.matches("https://a.com/extra"));
    }

    #[test]
    fn test_url_pattern_regex() {
        let p = UrlPattern::Regex(r"^https://.*\.google\.com/".to_string());
        assert!(p.matches("https://www.google.com/search"));
        assert!(!p.matches("https://example.com"));
    }

    #[test]
    fn test_url_pattern_regex_invalid() {
        let p = UrlPattern::Regex("[invalid".to_string());
        assert!(!p.matches("anything"));
    }

    #[test]
    fn test_url_pattern_wildcard() {
        let p = UrlPattern::Wildcard("*.google.com/*".to_string());
        assert!(p.matches("https://www.google.com/search"));
        assert!(!p.matches("https://example.com"));
    }

    #[test]
    fn test_url_pattern_wildcard_invalid_regex() {
        // Wildcard converts * to .* — this should still work
        let p = UrlPattern::Wildcard("*".to_string());
        assert!(p.matches("https://anything.com"));
    }

    #[test]
    fn test_mock_response_html() {
        let resp = MockResponse::html(200, "<h1>Hello</h1>");
        assert_eq!(resp.status, 200);
        assert_eq!(resp.body, b"<h1>Hello</h1>");
        assert!(resp.headers.iter().any(|(k, v)| k == "Content-Type" && v.contains("text/html")));
    }

    #[test]
    fn test_mock_response_text() {
        let resp = MockResponse::text(200, "plain text");
        assert_eq!(resp.status, 200);
        assert_eq!(resp.body, b"plain text");
        assert!(resp.headers.iter().any(|(k, v)| k == "Content-Type" && v.contains("text/plain")));
    }

    #[test]
    fn test_mock_response_json_body() {
        let resp = MockResponse::json(201, &json!({"created": true}));
        assert_eq!(resp.status, 201);
        let body_str = String::from_utf8(resp.body).unwrap();
        assert!(body_str.contains("created"));
    }

    #[test]
    fn test_ad_blocker_common_patterns() {
        let patterns = AdBlocker::common_patterns();
        assert!(patterns.contains(&"doubleclick.net"));
        assert!(patterns.contains(&"google-analytics.com"));
        assert!(patterns.contains(&"facebook.com/tr"));
        assert!(patterns.len() > 10);
    }

    #[test]
    fn test_intercept_action_clone() {
        let action = InterceptAction::Block;
        let cloned = action.clone();
        match cloned {
            InterceptAction::Block => {}
            _ => panic!("Expected Block"),
        }
    }

    #[test]
    fn test_intercept_action_continue() {
        let action = InterceptAction::Continue;
        let cloned = action.clone();
        match cloned {
            InterceptAction::Continue => {}
            _ => panic!("Expected Continue"),
        }
    }

    #[test]
    fn test_intercept_action_mock() {
        let resp = MockResponse::text(404, "not found");
        let action = InterceptAction::Mock(resp);
        let cloned = action.clone();
        match cloned {
            InterceptAction::Mock(r) => {
                assert_eq!(r.status, 404);
                assert_eq!(r.body, b"not found");
            }
            _ => panic!("Expected Mock"),
        }
    }

    #[test]
    fn test_intercept_action_modify_headers() {
        let headers = vec![("X-Custom".to_string(), "value".to_string())];
        let action = InterceptAction::ModifyHeaders(headers);
        let cloned = action.clone();
        match cloned {
            InterceptAction::ModifyHeaders(h) => {
                assert_eq!(h.len(), 1);
                assert_eq!(h[0].0, "X-Custom");
            }
            _ => panic!("Expected ModifyHeaders"),
        }
    }

    #[test]
    fn test_intercepted_request_clone() {
        let req = InterceptedRequest {
            request_id: "123".to_string(),
            url: "https://example.com".to_string(),
            method: "GET".to_string(),
            headers: json!({}),
            session_id: None,
        };
        let cloned = req.clone();
        assert_eq!(cloned.request_id, "123");
        assert_eq!(cloned.url, "https://example.com");
    }
}
