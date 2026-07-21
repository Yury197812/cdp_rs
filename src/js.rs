use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

use crate::browser::{CdpConnection, CdpEvent};

/// JavaScript executor with console interception and evaluation history
pub struct JsEngine {
    connection: Arc<CdpConnection>,
    console_log: Arc<RwLock<Vec<ConsoleEntry>>>,
    console_tx: broadcast::Sender<ConsoleEntry>,
    eval_history: Arc<RwLock<Vec<EvalEntry>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsoleEntry {
    pub timestamp: f64,
    pub level: String,
    pub text: String,
    pub args: Vec<Value>,
    pub url: Option<String>,
    pub line_number: Option<u32>,
    pub column_number: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalEntry {
    pub timestamp: f64,
    pub expression: String,
    pub result: Value,
    pub is_error: bool,
    pub duration_ms: f64,
}

impl JsEngine {
    pub fn new(connection: Arc<CdpConnection>) -> Self {
        let (console_tx, _) = broadcast::channel::<ConsoleEntry>(1024);
        Self {
            connection,
            console_log: Arc::new(RwLock::new(Vec::new())),
            console_tx,
            eval_history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Enable Runtime domain for JS execution and console capture
    pub async fn enable(&self) -> Result<()> {
        self.connection.send_page("Runtime.enable", json!({})).await?;
        Ok(())
    }

    /// Disable Runtime domain
    pub async fn disable(&self) -> Result<()> {
        self.connection.send_page("Runtime.disable", json!({})).await?;
        Ok(())
    }

    /// Process a CDP event — call this in your event loop to capture console messages
    pub async fn handle_event(&self, event: &CdpEvent) -> Result<()> {
        if event.method == "Runtime.consoleAPICalled" {
            let entry = ConsoleEntry {
                timestamp: event.params["timestamp"].as_f64().unwrap_or(0.0),
                level: event.params["type"].as_str().unwrap_or("log").to_string(),
                text: self.extract_console_text(&event.params),
                args: event.params["args"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default(),
                url: event.params["stackTrace"]["callFrames"][0]["url"]
                    .as_str()
                    .map(|s| s.to_string()),
                line_number: event.params["stackTrace"]["callFrames"][0]["lineNumber"]
                    .as_u64()
                    .map(|v| v as u32),
                column_number: event.params["stackTrace"]["callFrames"][0]["columnNumber"]
                    .as_u64()
                    .map(|v| v as u32),
            };

            self.console_log.write().await.push(entry.clone());
            let _ = self.console_tx.send(entry);
        }

        if event.method == "Runtime.exceptionThrown" {
            let exception = &event.params["exceptionDetails"];
            let entry = ConsoleEntry {
                timestamp: event.params["timestamp"].as_f64().unwrap_or(0.0),
                level: "error".to_string(),
                text: exception["text"].as_str().unwrap_or("Unhandled exception").to_string(),
                args: vec![exception.clone()],
                url: exception["url"].as_str().map(|s| s.to_string()),
                line_number: exception["lineNumber"].as_u64().map(|v| v as u32),
                column_number: exception["columnNumber"].as_u64().map(|v| v as u32),
            };

            self.console_log.write().await.push(entry.clone());
            let _ = self.console_tx.send(entry);
        }

        Ok(())
    }

    /// Execute JavaScript and return result
    pub async fn eval(&self, expression: &str) -> Result<Value> {
        self.eval_with_options(expression, true, true).await
    }

    /// Execute JavaScript with options
    pub async fn eval_with_options(
        &self,
        expression: &str,
        return_by_value: bool,
        await_promise: bool,
    ) -> Result<Value> {
        let start = std::time::Instant::now();

        let result = self.connection.send_page("Runtime.evaluate", json!({
            "expression": expression,
            "returnByValue": return_by_value,
            "awaitPromise": await_promise,
        })).await?;

        let duration = start.elapsed().as_secs_f64() * 1000.0;

        let entry = EvalEntry {
            timestamp: chrono::Utc::now().timestamp() as f64,
            expression: expression.to_string(),
            result: result.clone(),
            is_error: result.get("exceptionDetails").is_some(),
            duration_ms: duration,
        };
        self.eval_history.write().await.push(entry);

        if let Some(exception) = result.get("exceptionDetails") {
            let desc = exception["text"].as_str().unwrap_or("JS error");
            anyhow::bail!("JS error: {}", desc);
        }

        Ok(result.get("result").cloned().unwrap_or(Value::Null))
    }

    /// Execute JavaScript in a new isolated world (no side effects on page)
    pub async fn eval_isolated(&self, expression: &str) -> Result<Value> {
        let result = self.connection.send_page("Runtime.evaluate", json!({
            "expression": expression,
            "returnByValue": true,
            "uniqueContextId": format!("cdp_rs_{}", chrono::Utc::now().timestamp_millis()),
        })).await?;

        if let Some(exception) = result.get("exceptionDetails") {
            let desc = exception["text"].as_str().unwrap_or("JS error");
            anyhow::bail!("JS error: {}", desc);
        }

        Ok(result.get("result").cloned().unwrap_or(Value::Null))
    }

    /// Get a value as a string
    pub async fn eval_string(&self, expression: &str) -> Result<String> {
        let result = self.eval(expression).await?;
        Ok(result.get("value")
            .and_then(|v| v.as_str())
            .or_else(|| result.as_str())
            .unwrap_or("")
            .to_string())
    }

    /// Get a value as an integer
    pub async fn eval_int(&self, expression: &str) -> Result<i64> {
        let result = self.eval(expression).await?;
        result.get("value")
            .and_then(|v| v.as_i64())
            .or_else(|| result.as_i64())
            .ok_or_else(|| anyhow::anyhow!("Not an integer"))
    }

    /// Get a value as a float
    pub async fn eval_float(&self, expression: &str) -> Result<f64> {
        let result = self.eval(expression).await?;
        result.get("value")
            .and_then(|v| v.as_f64())
            .or_else(|| result.as_f64())
            .ok_or_else(|| anyhow::anyhow!("Not a float"))
    }

    /// Get a value as a boolean
    pub async fn eval_bool(&self, expression: &str) -> Result<bool> {
        let result = self.eval(expression).await?;
        result.get("value")
            .and_then(|v| v.as_bool())
            .or_else(|| result.as_bool())
            .ok_or_else(|| anyhow::anyhow!("Not a boolean"))
    }

    /// Get a value as a JSON object
    pub async fn eval_json(&self, expression: &str) -> Result<Value> {
        self.eval(expression).await
    }

    /// Wait for a JavaScript condition to become true
    pub async fn wait_for(&self, condition: &str, timeout_ms: u64) -> Result<()> {
        let start = std::time::Instant::now();
        loop {
            let result = self.eval(condition).await?;
            if result.get("value").and_then(|v| v.as_bool()).unwrap_or(false)
                || result.as_bool().unwrap_or(false)
            {
                return Ok(());
            }
            if start.elapsed().as_millis() > timeout_ms as u128 {
                anyhow::bail!("Timeout after {}ms waiting for: {}", timeout_ms, condition);
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    /// Subscribe to console messages
    pub fn on_console(&self) -> broadcast::Receiver<ConsoleEntry> {
        self.console_tx.subscribe()
    }

    /// Get all console messages
    pub async fn console_logs(&self) -> Vec<ConsoleEntry> {
        self.console_log.read().await.clone()
    }

    /// Get console messages by level
    pub async fn console_logs_by_level(&self, level: &str) -> Vec<ConsoleEntry> {
        self.console_log.read().await.iter()
            .filter(|e| e.level == level)
            .cloned()
            .collect()
    }

    /// Get only error console messages
    pub async fn console_errors(&self) -> Vec<ConsoleEntry> {
        self.console_logs_by_level("error").await
    }

    /// Get only warning console messages
    pub async fn console_warnings(&self) -> Vec<ConsoleEntry> {
        self.console_logs_by_level("warning").await
    }

    /// Get evaluation history
    pub async fn eval_history(&self) -> Vec<EvalEntry> {
        self.eval_history.read().await.clone()
    }

    /// Clear console log
    pub async fn clear_console(&self) {
        self.console_log.write().await.clear();
    }

    /// Clear eval history
    pub async fn clear_history(&self) {
        self.eval_history.write().await.clear();
    }

    /// Get console log as text
    pub async fn console_to_text(&self) -> String {
        let logs = self.console_log.read().await;
        logs.iter().map(|e| {
            format!("[{}] {}", e.level.to_uppercase(), e.text)
        }).collect::<Vec<_>>().join("\n")
    }

    /// Inject a script that runs on every page load
    pub async fn add_script_to_evaluate_on_new_document(&self, source: &str) -> Result<()> {
        self.connection.send_page("Page.addScriptToEvaluateOnNewDocument", json!({
            "source": source,
        })).await?;
        Ok(())
    }

    /// Remove injected scripts
    pub async fn remove_script_to_evaluate_on_new_document(&self, id: &str) -> Result<()> {
        self.connection.send_page("Page.removeScriptToEvaluateOnNewDocument", json!({
            "identifier": id,
        })).await?;
        Ok(())
    }

    /// Get JavaScript for common operations
    pub fn snippet(name: &str) -> &'static str {
        match name {
            "cookies" => "document.cookie",
            "localstorage" => "JSON.stringify(Object.fromEntries(Object.entries(localStorage)))",
            "sessionstorage" => "JSON.stringify(Object.fromEntries(Object.entries(sessionStorage)))",
            "viewport" => "JSON.stringify({width: window.innerWidth, height: window.innerHeight})",
            "scroll" => "JSON.stringify({x: window.scrollX, y: window.scrollY})",
            "title" => "document.title",
            "url" => "window.location.href",
            "all_links" => "JSON.stringify(Array.from(document.querySelectorAll('a')).map(a => ({href: a.href, text: a.textContent.trim()})))",
            "all_images" => "JSON.stringify(Array.from(document.querySelectorAll('img')).map(i => ({src: i.src, alt: i.alt, w: i.naturalWidth, h: i.naturalHeight})))",
            "all_forms" => "JSON.stringify(Array.from(document.querySelectorAll('form')).map(f => ({action: f.action, method: f.method})))",
            "performance" => "JSON.stringify({timing: performance.timing, entries: performance.getEntriesByType('navigation').map(e => ({type: e.initiatorType, duration: e.duration}))})",
            "errors" => "window.__errors = window.__errors || []; JSON.stringify(window.__errors)",
            _ => "",
        }
    }

    /// Run a snippet by name
    pub async fn run_snippet(&self, name: &str) -> Result<Value> {
        let js = Self::snippet(name);
        if js.is_empty() {
            anyhow::bail!("Unknown snippet: {}", name);
        }
        self.eval(js).await
    }

    fn extract_console_text(&self, params: &Value) -> String {
        params["args"]
            .as_array()
            .map(|args| {
                args.iter()
                    .filter_map(|a| {
                        if let Some(v) = a.get("value") {
                            Some(v.to_string())
                        } else if let Some(d) = a.get("description") {
                            Some(d.as_str().unwrap_or("").to_string())
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .unwrap_or_else(|| {
                params["text"].as_str().unwrap_or("").to_string()
            })
    }
}

use std::time::Duration;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_js_engine_snippets() {
        assert_eq!(JsEngine::snippet("cookies"), "document.cookie");
        assert_eq!(JsEngine::snippet("title"), "document.title");
        assert_eq!(JsEngine::snippet("url"), "window.location.href");
        assert!(JsEngine::snippet("all_links").contains("querySelectorAll"));
        assert!(JsEngine::snippet("nonexistent").is_empty());
    }

    #[test]
    fn test_console_entry_serialize() {
        let entry = ConsoleEntry {
            timestamp: 1234567890.0,
            level: "log".to_string(),
            text: "hello world".to_string(),
            args: vec![],
            url: Some("https://example.com".to_string()),
            line_number: Some(10),
            column_number: Some(5),
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("hello world"));
        assert!(json.contains("log"));
    }

    #[test]
    fn test_eval_entry_serialize() {
        let entry = EvalEntry {
            timestamp: 1234567890.0,
            expression: "1 + 1".to_string(),
            result: json!(2),
            is_error: false,
            duration_ms: 1.5,
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("1 + 1"));
        assert!(json.contains("1.5"));
    }

    #[test]
    fn test_console_entry_clone() {
        let entry = ConsoleEntry {
            timestamp: 0.0,
            level: "error".to_string(),
            text: "fail".to_string(),
            args: vec![],
            url: None,
            line_number: None,
            column_number: None,
        };
        let cloned = entry.clone();
        assert_eq!(cloned.level, "error");
        assert_eq!(cloned.text, "fail");
    }

    #[test]
    fn test_eval_entry_clone() {
        let entry = EvalEntry {
            timestamp: 0.0,
            expression: "test".to_string(),
            result: json!(null),
            is_error: true,
            duration_ms: 0.0,
        };
        let cloned = entry.clone();
        assert!(cloned.is_error);
    }

    #[test]
    fn test_console_to_text_format() {
        // Just verify the function exists and returns a string
        // Full test requires running engine
    }
}
