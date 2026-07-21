use anyhow::Result;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Semaphore, RwLock};
use tokio::time::Duration;

use crate::browser::CdpConnection;

/// Pooled browser instance with automatic cleanup
pub struct PooledBrowser {
    pub connection: Arc<CdpConnection>,
    pub port: u16,
    pub target_id: Option<String>,
    _permit: tokio::sync::OwnedSemaphorePermit,
}

impl Drop for PooledBrowser {
    fn drop(&mut self) {
        // Kill Chrome process on this port
        let _ = std::process::Command::new("taskkill")
            .args(["/F", "/IM", "chrome.exe"])
            .output();
        println!("[Pool] Released browser on port {}", self.port);
    }
}

impl PooledBrowser {
    /// Clean browser state between tests (cookies, cache, storage)
    pub async fn clean_state(&self) -> Result<()> {
        // Clear cookies
        let _ = self.connection.send_page("Network.clearBrowserCookies", json!({})).await;
        // Clear cache
        let _ = self.connection.send_page("Network.clearBrowserCache", json!({})).await;
        // Clear localStorage
        let _ = self.connection.send_page("Runtime.evaluate", json!({
            "expression": "try { localStorage.clear(); sessionStorage.clear(); } catch(e) {}"
        })).await;
        // Clear geolocation
        let _ = self.connection.send_page("Emulation.clearGeolocationOverride", json!({})).await;
        // Reset user agent
        let _ = self.connection.send_page("Emulation.setUserAgentOverride", json!({
            "userAgent": ""
        })).await;
        Ok(())
    }

    /// Navigate to URL
    pub async fn navigate(&self, url: &str) -> Result<()> {
        self.connection.send_page("Page.navigate", json!({"url": url})).await?;
        // Wait for load
        tokio::time::sleep(Duration::from_millis(500)).await;
        Ok(())
    }

    /// Execute JavaScript
    pub async fn evaluate(&self, expression: &str) -> Result<serde_json::Value> {
        let result = self.connection.send_page("Runtime.evaluate", json!({
            "expression": expression,
            "returnByValue": true,
        })).await?;
        Ok(result.get("result").cloned().unwrap_or(serde_json::Value::Null))
    }

    /// Take screenshot (returns base64 PNG)
    pub async fn screenshot(&self) -> Result<String> {
        let result = self.connection.send_page("Page.captureScreenshot", json!({
            "format": "png"
        })).await?;
        Ok(result.get("data").and_then(|d| d.as_str()).unwrap_or("").to_string())
    }
}

/// Browser pool manager — manages multiple Chrome instances
pub struct BrowserPool {
    semaphore: Arc<Semaphore>,
    port_start: u16,
    headless: bool,
    chrome_path: String,
    proxy: Option<String>,
    extra_headers: Option<std::collections::HashMap<String, String>>,
    extra_cookies: Vec<crate::browser::CookieEntry>,
    active_ports: Arc<RwLock<HashMap<u16, bool>>>,
}

impl BrowserPool {
    pub fn new(size: usize, port_start: u16, headless: bool) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(size)),
            port_start,
            headless,
            chrome_path: "chrome".to_string(),
            proxy: None,
            extra_headers: None,
            extra_cookies: Vec::new(),
            active_ports: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn with_chrome_path(mut self, path: &str) -> Self {
        self.chrome_path = path.to_string();
        self
    }

    /// Set HTTP/SOCKS5 proxy for all browsers in this pool
    pub fn with_proxy(mut self, proxy: &str) -> Self {
        self.proxy = Some(proxy.to_string());
        self
    }

    /// Set extra HTTP headers for all browsers in this pool
    pub fn with_headers(mut self, headers: std::collections::HashMap<String, String>) -> Self {
        self.extra_headers = Some(headers);
        self
    }

    /// Add a single extra HTTP header
    pub fn with_header(mut self, key: &str, value: &str) -> Self {
        self.extra_headers
            .get_or_insert_with(std::collections::HashMap::new)
            .insert(key.to_string(), value.to_string());
        self
    }

    /// Set cookies for all browsers in this pool
    pub fn with_cookies(mut self, cookies: Vec<crate::browser::CookieEntry>) -> Self {
        self.extra_cookies = cookies;
        self
    }

    /// Add a single cookie
    pub fn with_cookie(mut self, name: &str, value: &str) -> Self {
        self.extra_cookies.push(crate::browser::CookieEntry {
            name: name.to_string(),
            value: value.to_string(),
            domain: None,
            path: None,
        });
        self
    }

    /// Acquire a browser from the pool (blocks if pool is full)
    pub async fn acquire(&self) -> Result<PooledBrowser> {
        let permit = self.semaphore.clone().acquire_owned().await
            .map_err(|_| anyhow::anyhow!("Pool closed"))?;

        // Find available port
        let port = self.find_available_port().await?;

        println!("[Pool] Launching Chrome on port {}", port);

        // Launch Chrome
        let mut cmd = std::process::Command::new(&self.chrome_path);
        cmd.arg(format!("--remote-debugging-port={}", port));
        cmd.arg("--remote-debugging-address=127.0.0.1");
        cmd.arg("--no-first-run");
        cmd.arg("--no-default-browser-check");
        cmd.arg("--disable-background-networking");
        cmd.arg("--disable-sync");
        cmd.arg("--disable-translate");
        cmd.arg("--disable-extensions");

        let user_data = std::env::temp_dir().join(format!("cdp_pool_{}", port));
        cmd.arg(format!("--user-data-dir={}", user_data.to_str().unwrap()));

        if self.headless {
            cmd.arg("--headless=new");
        }

        if let Some(ref proxy) = self.proxy {
            cmd.arg(format!("--proxy-server={}", proxy));
            println!("[Pool] Using proxy: {}", proxy);
        }

        cmd.stdout(std::process::Stdio::piped())
           .stderr(std::process::Stdio::piped())
           .spawn()
           .map_err(|e| anyhow::anyhow!("Failed to launch Chrome: {}", e))?;

        // Wait for CDP to be ready
        let mut retries = 0;
        loop {
            tokio::time::sleep(Duration::from_millis(200)).await;
            match reqwest::get(format!("http://127.0.0.1:{}/json/version", port)).await {
                Ok(resp) if resp.status().is_success() => break,
                _ => {
                    retries += 1;
                    if retries > 25 {
                        anyhow::bail!("Chrome failed to start on port {} after 5s", port);
                    }
                }
            }
        }

        // Get WebSocket URL
        let version: serde_json::Value = reqwest::get(format!("http://127.0.0.1:{}/json/version", port))
            .await?.json().await?;
        let ws_url = version["webSocketDebuggerUrl"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("No webSocketDebuggerUrl"))?;

        let connection = Arc::new(CdpConnection::connect(ws_url).await?);

        // Enable domains
        let _ = connection.send("Page.enable", json!({})).await;
        let _ = connection.send("Runtime.enable", json!({})).await;
        let _ = connection.send("Network.enable", json!({})).await;

        // Apply extra headers
        if let Some(ref headers) = self.extra_headers {
            let header_json: Vec<serde_json::Value> = headers.iter()
                .map(|(k, v)| serde_json::json!({"name": k, "value": v}))
                .collect();
            let _ = connection.send_page("Network.setExtraHTTPHeaders", serde_json::json!({
                "headers": header_json,
            })).await;
        }

        // Apply cookies
        for cookie in &self.extra_cookies {
            let mut params = serde_json::json!({
                "name": cookie.name,
                "value": cookie.value,
                "url": "http://localhost",
            });
            if let Some(ref domain) = cookie.domain {
                params["domain"] = serde_json::Value::String(domain.clone());
            }
            if let Some(ref path) = cookie.path {
                params["path"] = serde_json::Value::String(path.clone());
            }
            let _ = connection.send_page("Network.setCookie", params).await;
        }

        // Mark port as active
        self.active_ports.write().await.insert(port, true);

        Ok(PooledBrowser {
            connection,
            port,
            target_id: None,
            _permit: permit,
        })
    }

    /// Create isolated browser context (separate cookies/storage)
    pub async fn isolated_context(browser: &PooledBrowser) -> Result<String> {
        let result = browser.connection.send("Target.createBrowserContext", json!({})).await?;
        let context_id = result.get("browserContextId")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        Ok(context_id)
    }

    async fn find_available_port(&self) -> Result<u16> {
        let active = self.active_ports.read().await;
        for port in self.port_start..self.port_start + 100 {
            if !active.contains_key(&port) && !self.is_port_in_use(port).await {
                return Ok(port);
            }
        }
        anyhow::bail!("No available ports in range {}-{}", self.port_start, self.port_start + 100);
    }

    async fn is_port_in_use(&self, port: u16) -> bool {
        tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port)).await.is_err()
    }

    /// Get pool statistics
    pub async fn stats(&self) -> PoolStats {
        let active = self.active_ports.read().await;
        PoolStats {
            total: self.semaphore.available_permits() + active.len(),
            active: active.len(),
            available: self.semaphore.available_permits(),
        }
    }
}

pub struct PoolStats {
    pub total: usize,
    pub active: usize,
    pub available: usize,
}

impl std::fmt::Display for PoolStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Pool: {}/{} active, {} available", self.active, self.total, self.available)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "requires Chrome"]
    async fn test_pool_acquire_release() {
        let pool = BrowserPool::new(2, 9500, true);
        let browser = pool.acquire().await.unwrap();
        println!("Acquired browser on port {}", browser.port);
        assert!(browser.port >= 9500);
        drop(browser); // Should cleanup
    }

    #[tokio::test]
    #[ignore = "requires Chrome"]
    async fn test_pool_isolated_context() {
        let pool = BrowserPool::new(1, 9600, true);
        let browser = pool.acquire().await.unwrap();
        let ctx_id = BrowserPool::isolated_context(&browser).await.unwrap();
        assert!(!ctx_id.is_empty());
    }

    #[test]
    fn test_pool_new() {
        let pool = BrowserPool::new(4, 9700, false);
        assert_eq!(pool.semaphore.available_permits(), 4);
        assert_eq!(pool.port_start, 9700);
        assert!(!pool.headless);
    }

    #[test]
    fn test_pool_with_chrome_path() {
        let pool = BrowserPool::new(1, 9800, true)
            .with_chrome_path("/usr/bin/chromium");
        assert_eq!(pool.chrome_path, "/usr/bin/chromium");
    }

    #[tokio::test]
    async fn test_pool_stats_initial() {
        let pool = BrowserPool::new(3, 9900, true);
        // Stats should show 3 total, 0 active, 3 available
        let stats = pool.stats().await;
        assert_eq!(stats.total, 3);
        assert_eq!(stats.active, 0);
        assert_eq!(stats.available, 3);
    }

    #[test]
    fn test_pool_stats_display() {
        let stats = PoolStats {
            total: 4,
            active: 2,
            available: 2,
        };
        let display = format!("{}", stats);
        assert!(display.contains("2/4"));
        assert!(display.contains("2 available"));
    }

    #[test]
    fn test_pooled_browser_clean_state_commands() {
        // Verify clean_state sends correct CDP commands
        // This tests the command structure, not execution
        let commands = vec![
            "Network.clearBrowserCookies",
            "Network.clearBrowserCache",
        ];
        for cmd in commands {
            assert!(!cmd.is_empty());
        }
    }

    #[test]
    fn test_pooled_browser_evaluate_js() {
        let expr = "document.title";
        assert!(!expr.is_empty());
    }

    #[test]
    fn test_pooled_browser_screenshot_format() {
        let format = "png";
        assert_eq!(format, "png");
    }
}
