use anyhow::Result;
use futures_util::StreamExt;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::{broadcast, oneshot, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Raw WebSocket CDP message
#[derive(Debug, Clone)]
pub enum CdpMessage {
    Response { id: u64, result: Value },
    Error { id: u64, code: i64, message: String },
    Event { method: String, params: Value, session_id: Option<String> },
}

#[derive(Clone, Debug)]
pub struct CdpEvent {
    pub method: String,
    pub params: Value,
    pub session_id: Option<String>,
}

/// WebSocket-based CDP client
pub struct CdpClient {
    ws_url: String,
    pending: Arc<Mutex<HashMap<u64, oneshot::Sender<CdpMessage>>>>,
    event_bus: broadcast::Sender<CdpEvent>,
    next_id: AtomicU64,
}

impl CdpClient {
    pub async fn connect(ws_url: &str) -> Result<Self> {
        let (event_bus, _) = broadcast::channel::<CdpEvent>(1024);
        let client = Self {
            ws_url: ws_url.to_string(),
            pending: Arc::new(Mutex::new(HashMap::new())),
            event_bus,
            next_id: AtomicU64::new(1),
        };
        client.spawn_reader().await?;
        Ok(client)
    }

    async fn spawn_reader(&self) -> Result<()> {
        // Connect via raw TCP + manual WebSocket upgrade for simplicity
        let url = url::Url::parse(&self.ws_url)?;
        let host = url.host_str().unwrap_or("127.0.0.1");
        let port = url.port().unwrap_or(9222);
        let path = url.path();

        let addr = format!("{}:{}", host, port);
        let mut stream = TcpStream::connect(&addr).await?;

        // WebSocket handshake
        let key = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &rand_bytes());
        let handshake = format!(
            "GET {} HTTP/1.1\r\nHost: {}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: {}\r\nSec-WebSocket-Version: 13\r\n\r\n",
            path, host, key
        );
        stream.write_all(handshake.as_bytes()).await?;

        // Read handshake response
        let mut buf = vec![0u8; 4096];
        let mut total = 0;
        loop {
            let n = stream.read(&mut buf[total..]).await?;
            total += n;
            let response = String::from_utf8_lossy(&buf[..total]);
            if response.contains("\r\n\r\n") {
                break;
            }
        }

        let pending = self.pending.clone();
        let event_bus = self.event_bus.clone();

        // Spawn reader task (manual WebSocket framing)
        tokio::spawn(async move {
            let mut buf = vec![0u8; 65536];
            let mut partial = Vec::new();
            loop {
                match stream.read(&mut buf).await {
                    Ok(0) => break,
                    Ok(n) => {
                        partial.extend_from_slice(&buf[..n]);
                        // Try to extract complete frames
                        while let Some((frame, consumed)) = parse_ws_frame(&partial) {
                            partial.drain(..consumed);
                            if let Ok(text) = String::from_utf8(frame) {
                                if let Ok(msg) = serde_json::from_str::<Value>(&text) {
                                    process_message(&msg, &pending, &event_bus).await;
                                }
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        Ok(())
    }

    pub fn subscribe(&self) -> broadcast::Receiver<CdpEvent> {
        self.event_bus.subscribe()
    }

    pub async fn send(&self, method: &str, params: Value) -> Result<Value> {
        self.send_with_session(method, params, None).await
    }

    pub async fn send_with_session(&self, method: &str, params: Value, session_id: Option<&str>) -> Result<Value> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = oneshot::channel();
        self.pending.lock().await.insert(id, tx);

        let mut msg = json!({
            "id": id,
            "method": method,
            "params": params,
        });
        if let Some(sid) = session_id {
            msg["sessionId"] = Value::String(sid.to_string());
        }

        // Send via WebSocket (we need a shared writer — simplified: reconnect each time)
        // In production, use a shared Sink. For now, use HTTP fallback for sending.
        let url = format!("http://127.0.0.1:{}/json", self.ws_url_to_port());
        let resp = reqwest::Client::new()
            .post(&url)
            .json(&msg)
            .send()
            .await?;

        let result: Value = resp.json().await?;
        if let Some(err) = result.get("error") {
            return Err(anyhow::anyhow!("CDP error: {}", err["message"].as_str().unwrap_or("unknown")));
        }

        // Wait for response via oneshot (with timeout)
        match tokio::time::timeout(std::time::Duration::from_secs(30), rx).await {
            Ok(Ok(CdpMessage::Response { result, .. })) => Ok(result),
            Ok(Ok(CdpMessage::Error { message, .. })) => Err(anyhow::anyhow!("CDP error: {}", message)),
            Ok(Ok(CdpMessage::Event { .. })) => Err(anyhow::anyhow!("Unexpected event in response channel")),
            Ok(Err(_)) => Err(anyhow::anyhow!("Channel closed")),
            Err(_) => {
                self.pending.lock().await.remove(&id);
                Err(anyhow::anyhow!("Timeout waiting for CDP response"))
            }
        }
    }

    fn ws_url_to_port(&self) -> u16 {
        self.ws_url.split(':').last()
            .and_then(|s| s.split('/').next())
            .and_then(|s| s.parse().ok())
            .unwrap_or(9222)
    }
}

fn rand_bytes() -> Vec<u8> {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().subsec_nanos();
    vec![
        (nanos >> 24) as u8,
        (nanos >> 16) as u8,
        (nanos >> 8) as u8,
        nanos as u8,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ]
}

fn parse_ws_frame(data: &[u8]) -> Option<(Vec<u8>, usize)> {
    if data.len() < 2 {
        return None;
    }
    let first = data[0];
    let second = data[1];
    let masked = (second & 0x80) != 0;
    let mut payload_len = (second & 0x7F) as usize;
    let mut offset = 2;

    if payload_len == 126 {
        if data.len() < 4 {
            return None;
        }
        payload_len = ((data[2] as usize) << 8) | (data[3] as usize);
        offset = 4;
    } else if payload_len == 127 {
        if data.len() < 10 {
            return None;
        }
        payload_len = ((data[2] as usize) << 56)
            | ((data[3] as usize) << 48)
            | ((data[4] as usize) << 40)
            | ((data[5] as usize) << 32)
            | ((data[6] as usize) << 24)
            | ((data[7] as usize) << 16)
            | ((data[8] as usize) << 8)
            | (data[9] as usize);
        offset = 10;
    }

    let mask_len = if masked { 4 } else { 0 };
    let total = offset + mask_len + payload_len;
    if data.len() < total {
        return None;
    }

    let mut payload = data[offset + mask_len..total].to_vec();
    if masked {
        let mask = &data[offset..offset + 4];
        for (i, byte) in payload.iter_mut().enumerate() {
            *byte ^= mask[i % 4];
        }
    }

    // Handle continuation frames (opcode 0) and text frames (opcode 1)
    let opcode = first & 0x0F;
    if opcode == 0 || opcode == 1 {
        return Some((payload, total));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_rand_bytes_length() {
        let bytes = rand_bytes();
        assert_eq!(bytes.len(), 16);
    }

    #[test]
    fn test_parse_ws_frame_empty() {
        assert_eq!(parse_ws_frame(&[]), None);
    }

    #[test]
    fn test_parse_ws_frame_single_byte() {
        assert_eq!(parse_ws_frame(&[0x81]), None);
    }

    #[test]
    fn test_parse_ws_frame_short_payload() {
        // Text frame, unmasked, payload "Hi" (2 bytes)
        let data = vec![0x81, 0x02, b'H', b'i'];
        let (payload, consumed) = parse_ws_frame(&data).unwrap();
        assert_eq!(payload, b"Hi");
        assert_eq!(consumed, 4);
    }

    #[test]
    fn test_parse_ws_frame_masked() {
        // Masked text frame, payload "OK"
        let mask = [0x12, 0x34, 0x56, 0x78];
        let original = b"OK";
        let masked: Vec<u8> = original.iter().enumerate().map(|(i, b)| b ^ mask[i % 4]).collect();
        let mut data = vec![0x81, 0x82]; // FIN + text, masked, len=2
        data.extend_from_slice(&mask);
        data.extend_from_slice(&masked);

        let (payload, consumed) = parse_ws_frame(&data).unwrap();
        assert_eq!(payload, b"OK");
        assert_eq!(consumed, 8); // 2 header + 4 mask + 2 payload
    }

    #[test]
    fn test_parse_ws_frame_medium_length() {
        // Payload length 126 (extended 16-bit), 100 bytes of 'A'
        let payload: Vec<u8> = vec![b'A'; 100];
        let mut data = vec![0x81, 126, 0, 100]; // FIN + text, len=126, 100
        data.extend_from_slice(&payload);

        let (parsed, consumed) = parse_ws_frame(&data).unwrap();
        assert_eq!(parsed.len(), 100);
        assert_eq!(consumed, 4 + 100);
    }

    #[test]
    fn test_parse_ws_frame_incomplete() {
        // Header says 10 bytes but only 5 available
        let data = vec![0x81, 0x0A, 1, 2, 3, 4, 5];
        assert_eq!(parse_ws_frame(&data), None);
    }

    #[test]
    fn test_parse_ws_frame_ping_ignored() {
        // Ping frame (opcode 9)
        let data = vec![0x89, 0x02, b'H', b'i'];
        assert_eq!(parse_ws_frame(&data), None);
    }

    #[test]
    fn test_ws_url_to_port() {
        let (event_bus, _) = broadcast::channel::<CdpEvent>(1);
        let client = CdpClient {
            ws_url: "ws://127.0.0.1:9222/devtools/browser/abc".to_string(),
            pending: Arc::new(Mutex::new(HashMap::new())),
            event_bus,
            next_id: AtomicU64::new(1),
        };
        assert_eq!(client.ws_url_to_port(), 9222);
    }

    #[test]
    fn test_ws_url_to_port_default() {
        let (event_bus, _) = broadcast::channel::<CdpEvent>(1);
        let client = CdpClient {
            ws_url: "ws://127.0.0.1/no-port".to_string(),
            pending: Arc::new(Mutex::new(HashMap::new())),
            event_bus,
            next_id: AtomicU64::new(1),
        };
        assert_eq!(client.ws_url_to_port(), 9222);
    }

    #[tokio::test]
    async fn test_process_message_response() {
        let pending: Arc<Mutex<HashMap<u64, oneshot::Sender<CdpMessage>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let (event_bus, _) = broadcast::channel::<CdpEvent>(1);

        let (tx, rx) = oneshot::channel();
        pending.lock().await.insert(1, tx);

        let msg = json!({"id": 1, "result": {"value": 42}});
        process_message(&msg, &pending, &event_bus).await;

        let result = rx.await.unwrap();
        match result {
            CdpMessage::Response { id, result } => {
                assert_eq!(id, 1);
                assert_eq!(result["value"], 42);
            }
            _ => panic!("Expected Response"),
        }
    }

    #[tokio::test]
    async fn test_process_message_error() {
        let pending: Arc<Mutex<HashMap<u64, oneshot::Sender<CdpMessage>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let (event_bus, _) = broadcast::channel::<CdpEvent>(1);

        let (tx, rx) = oneshot::channel();
        pending.lock().await.insert(2, tx);

        let msg = json!({"id": 2, "error": {"code": -32000, "message": "not found"}});
        process_message(&msg, &pending, &event_bus).await;

        let result = rx.await.unwrap();
        match result {
            CdpMessage::Error { id, code, message } => {
                assert_eq!(id, 2);
                assert_eq!(code, -32000);
                assert_eq!(message, "not found");
            }
            _ => panic!("Expected Error"),
        }
    }

    #[tokio::test]
    async fn test_process_message_event() {
        let pending: Arc<Mutex<HashMap<u64, oneshot::Sender<CdpMessage>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let (event_bus, _) = broadcast::channel::<CdpEvent>(1);
        let mut rx = event_bus.subscribe();

        let msg = json!({"method": "Page.loadEventFired", "params": {}, "sessionId": "sess1"});
        process_message(&msg, &pending, &event_bus).await;

        let event = rx.recv().await.unwrap();
        assert_eq!(event.method, "Page.loadEventFired");
        assert_eq!(event.session_id.as_deref(), Some("sess1"));
    }

    #[tokio::test]
    async fn test_process_message_unknown_id() {
        let pending: Arc<Mutex<HashMap<u64, oneshot::Sender<CdpMessage>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let (event_bus, _) = broadcast::channel::<CdpEvent>(1);

        // No pending sender for id=999 — should not panic
        let msg = json!({"id": 999, "result": {}});
        process_message(&msg, &pending, &event_bus).await;
    }

    #[test]
    fn test_cdp_event_clone() {
        let event = CdpEvent {
            method: "test".to_string(),
            params: json!({"key": "value"}),
            session_id: Some("s1".to_string()),
        };
        let cloned = event.clone();
        assert_eq!(cloned.method, "test");
        assert_eq!(cloned.session_id.as_deref(), Some("s1"));
    }

    #[test]
    fn test_cdp_message_clone() {
        let msg = CdpMessage::Response {
            id: 1,
            result: json!({"ok": true}),
        };
        let cloned = msg.clone();
        match cloned {
            CdpMessage::Response { id, result } => {
                assert_eq!(id, 1);
                assert_eq!(result["ok"], true);
            }
            _ => panic!("Expected Response"),
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
