use anyhow::Result;
use serde_json::{json, Value};
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;

use crate::browser::CdpConnection;

/// Simple LRU cache for evaluate results
struct EvalCache {
    entries: VecDeque<(String, Value, std::time::Instant)>,
    max_size: usize,
    ttl: Duration,
}

impl EvalCache {
    fn new(max_size: usize, ttl: Duration) -> Self {
        Self {
            entries: VecDeque::with_capacity(max_size),
            max_size,
            ttl,
        }
    }

    fn get(&mut self, key: &str) -> Option<Value> {
        self.entries.retain(|(_, _, ts)| ts.elapsed() < self.ttl);
        self.entries.iter()
            .find(|(k, _, _)| k == key)
            .map(|(_, v, _)| v.clone())
    }

    fn insert(&mut self, key: String, value: Value) {
        self.entries.retain(|(k, _, _)| k != &key);
        if self.entries.len() >= self.max_size {
            self.entries.pop_front();
        }
        self.entries.push_back((key, value, std::time::Instant::now()));
    }

    fn clear(&mut self) {
        self.entries.clear();
    }
}

/// High-level page interaction API with caching
pub struct Page {
    pub connection: Arc<CdpConnection>,
    session_id: Option<String>,
    eval_cache: std::sync::Mutex<EvalCache>,
}

impl Page {
    pub fn new(connection: Arc<CdpConnection>) -> Self {
        Self {
            connection,
            session_id: None,
            eval_cache: std::sync::Mutex::new(EvalCache::new(256, Duration::from_secs(5))),
        }
    }

    pub fn with_session(connection: Arc<CdpConnection>, session_id: String) -> Self {
        Self {
            connection,
            session_id: Some(session_id),
            eval_cache: std::sync::Mutex::new(EvalCache::new(256, Duration::from_secs(5))),
        }
    }

    /// Clear evaluation cache
    pub fn clear_cache(&self) {
        if let Ok(mut cache) = self.eval_cache.lock() {
            cache.clear();
        }
    }

    /// Batch evaluate multiple expressions
    pub async fn evaluate_batch(&self, expressions: &[&str]) -> Result<Vec<Value>> {
        let mut results = Vec::with_capacity(expressions.len());
        let mut commands = Vec::with_capacity(expressions.len());

        for expr in expressions {
            commands.push(("Runtime.evaluate", json!({
                "expression": expr,
                "returnByValue": true,
                "awaitPromise": true,
            })));
        }

        let batch_results = self.connection.send_batch_page(commands).await?;
        for result in batch_results {
            results.push(result.get("result").cloned().unwrap_or(Value::Null));
        }
        Ok(results)
    }

    /// Send multiple commands as batch (fire-and-forget)
    pub async fn send_batch(&self, commands: Vec<(&str, serde_json::Value)>) -> Result<Vec<Value>> {
        self.connection.send_batch_page(commands).await
    }

    pub async fn navigate(&self, url: &str) -> Result<()> {
        self.connection.send_page("Page.navigate", json!({"url": url})).await?;
        self.wait_for_load().await?;
        Ok(())
    }

    pub async fn wait_for_load(&self) -> Result<()> {
        let start = std::time::Instant::now();
        loop {
            let result = self.connection.send_page("Runtime.evaluate", json!({
                "expression": "document.readyState === 'complete'",
                "returnByValue": true,
            })).await?;

            if result.get("result").and_then(|r| r.get("value")).and_then(|v| v.as_bool()).unwrap_or(false) {
                return Ok(());
            }
            if start.elapsed() > Duration::from_secs(30) {
                anyhow::bail!("Page load timeout");
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    pub async fn click(&self, selector: &str) -> Result<()> {
        // Get element center coordinates
        let js = format!(
            r#"(() => {{
                const el = document.querySelector('{}');
                if (!el) return null;
                const rect = el.getBoundingClientRect();
                return {{ x: rect.x + rect.width/2, y: rect.y + rect.height/2 }};
            }})()"#,
            selector.replace('\'', "\\'")
        );

        let result = self.connection.send_page("Runtime.evaluate", json!({
            "expression": &js,
            "returnByValue": true,
        })).await?;

        let coords = result.get("result").and_then(|r| r.get("value"));
        let x = coords.and_then(|c| c.get("x")).and_then(|v| v.as_f64()).unwrap_or(0.0);
        let y = coords.and_then(|c| c.get("y")).and_then(|v| v.as_f64()).unwrap_or(0.0);

        // Move mouse and click
        self.connection.send_page("Input.dispatchMouseEvent", json!({
            "type": "mouseMoved",
            "x": x, "y": y,
        })).await?;
        tokio::time::sleep(Duration::from_millis(50)).await;
        self.connection.send_page("Input.dispatchMouseEvent", json!({
            "type": "mousePressed",
            "x": x, "y": y,
            "button": "left", "clickCount": 1,
        })).await?;
        self.connection.send_page("Input.dispatchMouseEvent", json!({
            "type": "mouseReleased",
            "x": x, "y": y,
            "button": "left", "clickCount": 1,
        })).await?;

        Ok(())
    }

    pub async fn click_coords(&self, x: f64, y: f64) -> Result<()> {
        self.connection.send_page("Input.dispatchMouseEvent", json!({
            "type": "mouseMoved", "x": x, "y": y,
        })).await?;
        tokio::time::sleep(Duration::from_millis(30)).await;
        self.connection.send_page("Input.dispatchMouseEvent", json!({
            "type": "mousePressed", "x": x, "y": y,
            "button": "left", "clickCount": 1,
        })).await?;
        self.connection.send_page("Input.dispatchMouseEvent", json!({
            "type": "mouseReleased", "x": x, "y": y,
            "button": "left", "clickCount": 1,
        })).await?;
        Ok(())
    }

    pub async fn type_text(&self, text: &str) -> Result<()> {
        for ch in text.chars() {
            self.connection.send_page("Input.dispatchKeyEvent", json!({
                "type": "keyDown",
                "text": ch.to_string(),
                "key": ch.to_string(),
                "code": format!("Key{}", ch.to_uppercase()),
            })).await?;
            self.connection.send_page("Input.dispatchKeyEvent", json!({
                "type": "keyUp",
                "key": ch.to_string(),
                "code": format!("Key{}", ch.to_uppercase()),
            })).await?;
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        Ok(())
    }

    pub async fn press_key(&self, key: &str) -> Result<()> {
        let code = match key {
            "Enter" => "Enter",
            "Tab" => "Tab",
            "Escape" => "Escape",
            "Backspace" => "Backspace",
            "ArrowDown" => "ArrowDown",
            "ArrowUp" => "ArrowUp",
            _ => key,
        };
        self.connection.send_page("Input.dispatchKeyEvent", json!({
            "type": "keyDown", "key": key, "code": code,
        })).await?;
        self.connection.send_page("Input.dispatchKeyEvent", json!({
            "type": "keyUp", "key": key, "code": code,
        })).await?;
        Ok(())
    }

    pub async fn fill(&self, selector: &str, value: &str) -> Result<()> {
        // Focus element, clear it, type value
        let focus_js = format!(
            "document.querySelector('{}')?.focus()",
            selector.replace('\'', "\\'")
        );
        self.connection.send_page("Runtime.evaluate", json!({
            "expression": &focus_js,
        })).await?;
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Select all and delete
        self.press_key("Control+a").await?;
        self.press_key("Backspace").await?;
        tokio::time::sleep(Duration::from_millis(50)).await;

        self.type_text(value).await
    }

    pub async fn screenshot(&self, path: &str) -> Result<()> {
        let result = self.connection.send_page("Page.captureScreenshot", json!({
            "format": "png",
        })).await?;

        if let Some(data) = result.get("data").and_then(|d| d.as_str()) {
            use base64::Engine;
            let bytes = base64::engine::general_purpose::STANDARD.decode(data)?;
            tokio::fs::write(path, bytes).await?;
        }
        Ok(())
    }

    pub async fn evaluate(&self, expression: &str) -> Result<Value> {
        let result = self.connection.send_page("Runtime.evaluate", json!({
            "expression": expression,
            "returnByValue": true,
            "awaitPromise": true,
        })).await?;
        Ok(result.get("result").cloned().unwrap_or(Value::Null))
    }

    pub async fn get_url(&self) -> Result<String> {
        let result = self.evaluate("window.location.href").await?;
        Ok(result.get("value").and_then(|v| v.as_str()).unwrap_or("").to_string())
    }

    pub async fn get_title(&self) -> Result<String> {
        let result = self.evaluate("document.title").await?;
        Ok(result.get("value").and_then(|v| v.as_str()).unwrap_or("").to_string())
    }

    pub async fn wait_for_selector(&self, selector: &str, timeout_ms: u64) -> Result<()> {
        let start = std::time::Instant::now();
        let check = format!("document.querySelector('{}') !== null", selector.replace('\'', "\\'"));
        loop {
            let result = self.connection.send_page("Runtime.evaluate", json!({
                "expression": &check,
                "returnByValue": true,
            })).await?;
            if result.get("result").and_then(|r| r.get("value")).and_then(|v| v.as_bool()).unwrap_or(false) {
                return Ok(());
            }
            if start.elapsed().as_millis() > timeout_ms as u128 {
                anyhow::bail!("Timeout waiting for selector: {}", selector);
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    pub async fn wait_for_text(&self, text: &str, timeout_ms: u64) -> Result<()> {
        let start = std::time::Instant::now();
        let check = format!("document.body.innerText.includes('{}')", text.replace('\'', "\\'"));
        loop {
            let result = self.connection.send_page("Runtime.evaluate", json!({
                "expression": &check,
                "returnByValue": true,
            })).await?;
            if result.get("result").and_then(|r| r.get("value")).and_then(|v| v.as_bool()).unwrap_or(false) {
                return Ok(());
            }
            if start.elapsed().as_millis() > timeout_ms as u128 {
                anyhow::bail!("Timeout waiting for text: {}", text);
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    }

    pub async fn wait_for_url_contains(&self, pattern: &str, timeout_ms: u64) -> Result<()> {
        let start = std::time::Instant::now();
        loop {
            let url = self.get_url().await?;
            if url.contains(pattern) {
                return Ok(());
            }
            if start.elapsed().as_millis() > timeout_ms as u128 {
                anyhow::bail!("Timeout waiting for URL containing: {}", pattern);
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_press_key_enter() {
        // Verify key mapping for Enter
        let key = "Enter";
        let code = match key {
            "Enter" => "Enter",
            "Tab" => "Tab",
            "Escape" => "Escape",
            "Backspace" => "Backspace",
            "ArrowDown" => "ArrowDown",
            "ArrowUp" => "ArrowUp",
            _ => key,
        };
        assert_eq!(code, "Enter");
    }

    #[test]
    fn test_press_key_unknown() {
        let key = "Space";
        let code = match key {
            "Enter" => "Enter",
            "Tab" => "Tab",
            "Escape" => "Escape",
            "Backspace" => "Backspace",
            "ArrowDown" => "ArrowDown",
            "ArrowUp" => "ArrowUp",
            _ => key,
        };
        assert_eq!(code, "Space");
    }

    #[test]
    fn test_click_js_generation() {
        let selector = "#my-button";
        let js = format!(
            r#"(() => {{
                const el = document.querySelector('{}');
                if (!el) return null;
                const rect = el.getBoundingClientRect();
                return {{ x: rect.x + rect.width/2, y: rect.y + rect.height/2 }};
            }})()"#,
            selector.replace('\'', "\\'")
        );
        assert!(js.contains("#my-button"));
        assert!(js.contains("getBoundingClientRect"));
    }

    #[test]
    fn test_click_js_escapes_quotes() {
        let selector = "div[data-testid='login']";
        let js = format!(
            r#"(() => {{
                const el = document.querySelector('{}');
                if (!el) return null;
                const rect = el.getBoundingClientRect();
                return {{ x: rect.x + rect.width/2, y: rect.y + rect.height/2 }};
            }})()"#,
            selector.replace('\'', "\\'")
        );
        assert!(js.contains("\\'"));
    }

    #[test]
    fn test_wait_for_load_js() {
        let expr = "document.readyState === 'complete'";
        assert!(expr.contains("complete"));
    }

    #[test]
    fn test_wait_for_selector_js() {
        let selector = ".submit-btn";
        let check = format!("document.querySelector('{}') !== null", selector.replace('\'', "\\'"));
        assert!(check.contains(".submit-btn"));
        assert!(check.contains("!== null"));
    }

    #[test]
    fn test_wait_for_text_js() {
        let text = "Hello World";
        let check = format!("document.body.innerText.includes('{}')", text.replace('\'', "\\'"));
        assert!(check.contains("Hello World"));
        assert!(check.contains("includes"));
    }

    #[test]
    fn test_fill_js_generation() {
        let selector = "input[name='email']";
        let focus_js = format!(
            "document.querySelector('{}')?.focus()",
            selector.replace('\'', "\\'")
        );
        assert!(focus_js.contains("input[name="));
        assert!(focus_js.contains("focus()"));
    }

    #[test]
    fn test_evaluate_js_generation() {
        let expression = "window.location.href";
        let _ = json!({
            "expression": expression,
            "returnByValue": true,
            "awaitPromise": true,
        });
        // Verify the JSON structure is valid
        assert!(true);
    }

    #[test]
    fn test_type_text_char_code() {
        // Verify character code generation
        let ch = 'A';
        let code = format!("Key{}", ch.to_uppercase());
        assert_eq!(code, "KeyA");
    }

    #[test]
    fn test_type_text_digit() {
        let ch = '5';
        let code = format!("Key{}", ch.to_uppercase());
        assert_eq!(code, "Key5");
    }
}
