use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use std::collections::HashMap;
use std::net::ToSocketAddrs;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc, oneshot, Mutex};
use tokio_tungstenite::{connect_async, tungstenite::Message};

// ============================================================
// CDP Event types
// ============================================================
#[derive(Clone, Debug)]
pub struct CdpEvent {
    pub method: String,
    pub params: serde_json::Value,
    pub session_id: Option<String>,
}

#[derive(Clone, Debug)]
pub enum CdpMessage {
    Response { id: u64, result: serde_json::Value },
    Error { id: u64, code: i64, message: String },
}

// ============================================================
// CDP Connection via WebSocket
// ============================================================
pub struct CdpConnection {
    pending: Arc<Mutex<HashMap<u64, oneshot::Sender<CdpMessage>>>>,
    event_bus: broadcast::Sender<CdpEvent>,
    next_id: AtomicU64,
    ws_sender: Arc<Mutex<futures_util::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>
        >,
        Message,
    >>>,
    current_session: Arc<Mutex<Option<String>>>,
}

impl CdpConnection {
    /// Create a connection by connecting to a WebSocket URL
    pub async fn connect(ws_url: &str) -> Result<Self> {
        let (ws_stream, _) = connect_async(ws_url).await?;
        let (ws_sender, mut ws_receiver) = ws_stream.split();

        let pending: Arc<Mutex<HashMap<u64, oneshot::Sender<CdpMessage>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let (event_bus, _) = broadcast::channel::<CdpEvent>(1024);

        let pending_clone = pending.clone();
        let event_bus_clone = event_bus.clone();

        // Spawn reader task
        tokio::spawn(async move {
            while let Some(msg) = ws_receiver.next().await {
                match msg {
                    Ok(Message::Text(text)) => {
                        if let Ok(json) = serde_json::from_str::<Value>(&text) {
                            process_message(&json, &pending_clone, &event_bus_clone).await;
                        }
                    }
                    Ok(Message::Close(_)) => break,
                    Err(_) => break,
                    _ => {}
                }
            }
        });

        let conn = Self {
            pending,
            event_bus,
            next_id: AtomicU64::new(1),
            ws_sender: Arc::new(Mutex::new(ws_sender)),
            current_session: Arc::new(Mutex::new(None)),
        };

        // Create a page target and attach to it
        let result = conn.send("Target.createTarget", serde_json::json!({
            "url": "about:blank"
        })).await?;
        let target_id = result.get("targetId")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("No targetId in createTarget response"))?
            .to_string();

        let attach_result = conn.send("Target.attachToTarget", serde_json::json!({
            "targetId": &target_id,
            "flatten": true,
        })).await?;
        let session_id = attach_result.get("sessionId")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("No sessionId in attachToTarget response"))?
            .to_string();

        // Store session ID for subsequent commands
        conn.set_session(session_id).await;

        // Enable Page domain
        conn.send_page("Page.enable", serde_json::json!({})).await?;
        conn.send_page("Runtime.enable", serde_json::json!({})).await?;

        Ok(conn)
    }

    async fn set_session(&self, session_id: String) {
        *self.current_session.lock().await = Some(session_id);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<CdpEvent> {
        self.event_bus.subscribe()
    }

    /// Send command and don't wait for response (fire-and-forget)
    pub async fn send_no_wait(&self, method: &str, params: Value) -> Result<u64> {
        self.send_no_wait_with_session(method, params, None).await
    }

    pub async fn send_no_wait_with_session(&self, method: &str, params: Value, session_id: Option<&str>) -> Result<u64> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);

        let mut msg = serde_json::json!({
            "id": id,
            "method": method,
            "params": params,
        });
        if let Some(sid) = session_id {
            msg["sessionId"] = Value::String(sid.to_string());
        }

        let mut sender = self.ws_sender.lock().await;
        sender.send(Message::Text(msg.to_string().into())).await?;
        drop(sender);

        Ok(id)
    }

    /// Send batch of commands (fire-and-forget for all, collect responses)
    pub async fn send_batch(&self, commands: Vec<(&str, Value)>) -> Result<Vec<Value>> {
        let mut results = Vec::with_capacity(commands.len());
        let mut futures = Vec::with_capacity(commands.len());

        for (method, params) in commands {
            futures.push(self.send(method, params));
        }

        for fut in futures {
            match fut.await {
                Ok(v) => results.push(v),
                Err(e) => results.push(serde_json::json!({"error": e.to_string()})),
            }
        }

        Ok(results)
    }

    /// Batch send with session (for page-level commands)
    pub async fn send_batch_page(&self, commands: Vec<(&str, Value)>) -> Result<Vec<Value>> {
        let session = self.current_session.lock().await;
        let mut results = Vec::with_capacity(commands.len());

        for (method, params) in commands {
            match self.send_with_session(method, params, session.as_deref()).await {
                Ok(v) => results.push(v),
                Err(e) => results.push(serde_json::json!({"error": e.to_string()})),
            }
        }

        Ok(results)
    }

    pub async fn send(&self, method: &str, params: Value) -> Result<Value> {
        self.send_with_session(method, params, None).await
    }

    pub async fn send_page(&self, method: &str, params: Value) -> Result<Value> {
        let session = self.current_session.lock().await;
        self.send_with_session(method, params, session.as_deref()).await
    }

    pub async fn send_with_session(&self, method: &str, params: Value, session_id: Option<&str>) -> Result<Value> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = oneshot::channel();
        self.pending.lock().await.insert(id, tx);

        let mut msg = serde_json::json!({
            "id": id,
            "method": method,
            "params": params,
        });
        if let Some(sid) = session_id {
            msg["sessionId"] = Value::String(sid.to_string());
        }

        // Send via WebSocket
        let mut sender = self.ws_sender.lock().await;
        sender.send(Message::Text(msg.to_string().into())).await?;
        drop(sender);

        // Wait for response (with timeout)
        match tokio::time::timeout(Duration::from_secs(30), rx).await {
            Ok(Ok(CdpMessage::Response { result, .. })) => Ok(result),
            Ok(Ok(CdpMessage::Error { message, .. })) => {
                Err(anyhow::anyhow!("CDP error: {}", message))
            }
            Ok(Err(_)) => Err(anyhow::anyhow!("Channel closed")),
            Err(_) => {
                self.pending.lock().await.remove(&id);
                Err(anyhow::anyhow!("Timeout waiting for CDP response"))
            }
        }
    }
}

async fn process_message(
    msg: &Value,
    pending: &Arc<Mutex<HashMap<u64, oneshot::Sender<CdpMessage>>>>,
    event_bus: &broadcast::Sender<CdpEvent>,
) {
    if let Some(id) = msg.get("id").and_then(|v| v.as_u64()) {
        let mut pending = pending.lock().await;
        if let Some(tx) = pending.remove(&id) {
            if let Some(error) = msg.get("error") {
                let _ = tx.send(CdpMessage::Error {
                    id,
                    code: error.get("code").and_then(|v| v.as_i64()).unwrap_or(-1),
                    message: error.get("message").and_then(|v| v.as_str()).unwrap_or("unknown").to_string(),
                });
            } else {
                let _ = tx.send(CdpMessage::Response {
                    id,
                    result: msg.get("result").cloned().unwrap_or(Value::Null),
                });
            }
        }
    } else if let Some(method) = msg.get("method").and_then(|v| v.as_str()) {
        let event = CdpEvent {
            method: method.to_string(),
            params: msg.get("params").cloned().unwrap_or(Value::Null),
            session_id: msg.get("sessionId").and_then(|v| v.as_str()).map(|s| s.to_string()),
        };
        let _ = event_bus.send(event);
    }
}

// ============================================================
// Browser Manager with security improvements
// ============================================================
pub struct BrowserManager {
    binary: String,
    port: u16,
    proxy: Option<String>,
    extra_headers: Option<std::collections::HashMap<String, String>>,
    extra_cookies: Vec<CookieEntry>,
    child: Option<Child>,
    connection: Option<Arc<CdpConnection>>,
    connected: bool,
}

#[derive(Clone, Debug)]
pub struct CookieEntry {
    pub name: String,
    pub value: String,
    pub domain: Option<String>,
    pub path: Option<String>,
}

impl BrowserManager {
    pub fn new() -> Self {
        Self {
            binary: "chrome".to_string(),
            port: 0,
            proxy: None,
            extra_headers: None,
            extra_cookies: Vec::new(),
            child: None,
            connection: None,
            connected: false,
        }
    }

    pub fn binary(mut self, binary: &str) -> Self {
        self.binary = binary.to_string();
        self
    }

    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Set HTTP/SOCKS5 proxy for the browser
    /// Format: "http://host:port", "socks5://host:port", "http://user:pass@host:port"
    pub fn proxy(mut self, proxy: &str) -> Self {
        self.proxy = Some(proxy.to_string());
        self
    }

    /// Set extra HTTP headers sent with every request
    pub fn headers(mut self, headers: std::collections::HashMap<String, String>) -> Self {
        self.extra_headers = Some(headers);
        self
    }

    /// Add a single extra HTTP header
    pub fn header(mut self, key: &str, value: &str) -> Self {
        self.extra_headers
            .get_or_insert_with(std::collections::HashMap::new)
            .insert(key.to_string(), value.to_string());
        self
    }

    /// Set cookies to be sent with every request
    pub fn cookies(mut self, cookies: Vec<CookieEntry>) -> Self {
        self.extra_cookies = cookies;
        self
    }

    /// Add a single cookie
    pub fn cookie(mut self, name: &str, value: &str) -> Self {
        self.extra_cookies.push(CookieEntry {
            name: name.to_string(),
            value: value.to_string(),
            domain: None,
            path: None,
        });
        self
    }

    /// Add a cookie with domain and path
    pub fn cookie_with_domain(mut self, name: &str, value: &str, domain: &str, path: &str) -> Self {
        self.extra_cookies.push(CookieEntry {
            name: name.to_string(),
            value: value.to_string(),
            domain: Some(domain.to_string()),
            path: Some(path.to_string()),
        });
        self
    }

    fn random_port() -> u16 {
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .subsec_nanos();
        49152 + (nanos % 16383) as u16
    }

    pub async fn launch(&mut self) -> Result<Self> {
        if self.port == 0 {
            self.port = Self::random_port();
        }

        println!("[Browser] Launching {} on port {}", self.binary, self.port);

        if self.is_port_in_use().await {
            self.port = Self::random_port();
            println!("[Browser] Port in use, trying {}", self.port);
        }

        let user_data_dir = std::env::temp_dir().join(format!("cdp_rs_{}", self.port));

        let mut args = vec![
            format!("--remote-debugging-port={}", self.port),
            "--remote-debugging-address=127.0.0.1".to_string(),
            format!("--user-data-dir={}", user_data_dir.display()),
            "--no-first-run".to_string(),
            "--no-default-browser-check".to_string(),
            "--disable-background-networking".to_string(),
            "--disable-sync".to_string(),
            "--disable-translate".to_string(),
            "--disable-extensions".to_string(),
        ];

        if let Some(ref proxy) = self.proxy {
            args.push(format!("--proxy-server={}", proxy));
            println!("[Browser] Using proxy: {}", proxy);
        }

        let child = Command::new(&self.binary)
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| anyhow::anyhow!("Failed to launch browser: {}", e))?;

        println!("[Browser] Process started (PID: {})", child.id());

        // Wait for CDP HTTP endpoint
        self.wait_for_cdp_with_retry(10, Duration::from_millis(500)).await?;

        // Get WebSocket URL from /json/version
        let version_url = format!("http://127.0.0.1:{}/json/version", self.port);
        let version: Value = reqwest::get(&version_url).await?.json().await?;
        let ws_url = version["webSocketDebuggerUrl"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("No webSocketDebuggerUrl in /json/version"))?;

        println!("[Browser] WebSocket URL: {}", ws_url);

        let connection = Arc::new(CdpConnection::connect(ws_url).await?);

        println!("[Browser] CDP connected on port {}", self.port);

        // Apply extra headers via CDP
        if let Some(ref headers) = self.extra_headers {
            let header_json: Vec<Value> = headers.iter()
                .map(|(k, v)| serde_json::json!({"name": k, "value": v}))
                .collect();
            let _ = connection.send_page("Network.setExtraHTTPHeaders", serde_json::json!({
                "headers": header_json,
            })).await;
            println!("[Browser] Set {} extra headers", headers.len());
        }

        // Apply cookies via CDP
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
        if !self.extra_cookies.is_empty() {
            println!("[Browser] Set {} cookies", self.extra_cookies.len());
        }

        Ok(Self {
            binary: self.binary.clone(),
            port: self.port,
            proxy: self.proxy.clone(),
            extra_headers: self.extra_headers.clone(),
            extra_cookies: self.extra_cookies.clone(),
            child: Some(child),
            connection: Some(connection),
            connected: true,
        })
    }

    async fn wait_for_cdp_with_retry(&self, max_retries: u32, base_delay: Duration) -> Result<()> {
        let url = format!("http://127.0.0.1:{}/json/version", self.port);

        for attempt in 0..max_retries {
            match reqwest::get(&url).await {
                Ok(resp) if resp.status().is_success() => {
                    println!("[Browser] CDP ready after {} attempts", attempt + 1);
                    return Ok(());
                }
                _ => {
                    let delay = base_delay * 2u32.pow(attempt);
                    println!("[Browser] Retry {}/{} after {:?}", attempt + 1, max_retries, delay);
                    tokio::time::sleep(delay).await;
                }
            }
        }

        Err(anyhow::anyhow!("CDP timeout after {} retries", max_retries))
    }

    async fn is_port_in_use(&self) -> bool {
        format!("127.0.0.1:{}", self.port)
            .to_socket_addrs()
            .is_ok()
    }

    pub fn get_port(&self) -> u16 {
        self.port
    }

    pub fn connection(&self) -> Option<Arc<CdpConnection>> {
        self.connection.clone()
    }

    pub fn subscribe_events(&self) -> Option<broadcast::Receiver<CdpEvent>> {
        self.connection.as_ref().map(|c| c.subscribe())
    }
}

impl Drop for BrowserManager {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            println!("[Browser] WARNING: Browser not shut down cleanly, killing PID {}", child.id());
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_browser_manager_new() {
        let bm = BrowserManager::new();
        assert_eq!(bm.binary, "chrome");
        assert_eq!(bm.port, 0);
        assert!(bm.child.is_none());
        assert!(bm.connection.is_none());
        assert!(!bm.connected);
    }

    #[test]
    fn test_browser_manager_builder() {
        let bm = BrowserManager::new()
            .binary("chromium")
            .port(9333);
        assert_eq!(bm.binary, "chromium");
        assert_eq!(bm.port, 9333);
    }

    #[test]
    fn test_random_port_range() {
        let port = BrowserManager::random_port();
        assert!(port >= 49152);
    }

    #[test]
    fn test_random_port_different() {
        let p1 = BrowserManager::random_port();
        let p2 = BrowserManager::random_port();
        assert!(p1 >= 49152);
        assert!(p2 >= 49152);
    }

    #[test]
    fn test_cdp_event_clone() {
        let event = CdpEvent {
            method: "Page.loadEventFired".to_string(),
            params: json!({}),
            session_id: None,
        };
        let cloned = event.clone();
        assert_eq!(cloned.method, "Page.loadEventFired");
        assert!(cloned.session_id.is_none());
    }

    #[test]
    fn test_cdp_event_with_session() {
        let event = CdpEvent {
            method: "Runtime.consoleAPICalled".to_string(),
            params: json!({"type": "log"}),
            session_id: Some("sess_123".to_string()),
        };
        assert_eq!(event.session_id.as_deref(), Some("sess_123"));
    }

    #[test]
    fn test_cdp_message_response() {
        let msg = CdpMessage::Response {
            id: 1,
            result: json!({"value": "test"}),
        };
        match msg {
            CdpMessage::Response { id, result } => {
                assert_eq!(id, 1);
                assert_eq!(result["value"], "test");
            }
            _ => panic!("Expected Response"),
        }
    }

    #[test]
    fn test_cdp_message_error() {
        let msg = CdpMessage::Error {
            id: 2,
            code: -32000,
            message: "Error".to_string(),
        };
        match msg {
            CdpMessage::Error { id, code, message } => {
                assert_eq!(id, 2);
                assert_eq!(code, -32000);
                assert_eq!(message, "Error");
            }
            _ => panic!("Expected Error"),
        }
    }

    #[test]
    fn test_subscribe_events_no_connection() {
        let bm = BrowserManager::new();
        assert!(bm.subscribe_events().is_none());
    }
}
