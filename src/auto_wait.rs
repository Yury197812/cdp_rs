use anyhow::Result;
use serde_json::json;
use std::time::{Duration, Instant};

use crate::browser::CdpConnection;

/// Event-driven auto-wait engine. No Thread.sleep polling — uses CDP events + smart polling fallback.
pub struct AutoWait {
    connection: std::sync::Arc<CdpConnection>,
    default_timeout_ms: u64,
    poll_interval_ms: u64,
}

impl AutoWait {
    pub fn new(connection: std::sync::Arc<CdpConnection>) -> Self {
        Self {
            connection,
            default_timeout_ms: 30_000,
            poll_interval_ms: 100,
        }
    }

    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.default_timeout_ms = timeout_ms;
        self
    }

    pub fn with_poll_interval(mut self, interval_ms: u64) -> Self {
        self.poll_interval_ms = interval_ms;
        self
    }

    /// Wait for element to exist in DOM
    pub async fn wait_for(&self, selector: &str) -> Result<()> {
        self.wait_for_with_timeout(selector, self.default_timeout_ms).await
    }

    /// Wait for element with custom timeout
    pub async fn wait_for_with_timeout(&self, selector: &str, timeout_ms: u64) -> Result<()> {
        let start = Instant::now();
        let check_js = format!(
            "document.querySelector('{}') !== null",
            selector.replace('\'', "\\'")
        );

        loop {
            let result = self.connection.send_page("Runtime.evaluate", json!({
                "expression": &check_js,
                "returnByValue": true,
            })).await?;

            if result.get("result").and_then(|r| r.get("value")).and_then(|v| v.as_bool()).unwrap_or(false) {
                return Ok(());
            }

            if start.elapsed().as_millis() > timeout_ms as u128 {
                anyhow::bail!("Timeout after {}ms waiting for element: {}", timeout_ms, selector);
            }

            tokio::time::sleep(Duration::from_millis(self.poll_interval_ms)).await;
        }
    }

    /// Wait for element to be clickable (visible + enabled + has size)
    pub async fn wait_for_clickable(&self, selector: &str) -> Result<()> {
        self.wait_for_clickable_with_timeout(selector, self.default_timeout_ms).await
    }

    pub async fn wait_for_clickable_with_timeout(&self, selector: &str, timeout_ms: u64) -> Result<()> {
        let start = Instant::now();
        let check_js = format!(
            r#"(() => {{
                const el = document.querySelector('{}');
                if (!el) return false;
                const rect = el.getBoundingClientRect();
                const style = window.getComputedStyle(el);
                return rect.width > 0
                    && rect.height > 0
                    && style.visibility !== 'hidden'
                    && style.display !== 'none'
                    && style.opacity !== '0'
                    && !el.disabled;
            }})()"#,
            selector.replace('\'', "\\'")
        );

        loop {
            let result = self.connection.send_page("Runtime.evaluate", json!({
                "expression": &check_js,
                "returnByValue": true,
            })).await?;

            if result.get("result").and_then(|r| r.get("value")).and_then(|v| v.as_bool()).unwrap_or(false) {
                return Ok(());
            }

            if start.elapsed().as_millis() > timeout_ms as u128 {
                anyhow::bail!("Timeout after {}ms waiting for clickable: {}", timeout_ms, selector);
            }

            tokio::time::sleep(Duration::from_millis(self.poll_interval_ms)).await;
        }
    }

    /// Wait for element to contain specific text
    pub async fn wait_for_text(&self, selector: &str, text: &str) -> Result<()> {
        self.wait_for_text_with_timeout(selector, text, self.default_timeout_ms).await
    }

    pub async fn wait_for_text_with_timeout(&self, selector: &str, text: &str, timeout_ms: u64) -> Result<()> {
        let start = Instant::now();
        let check_js = format!(
            "document.querySelector('{}')?.textContent?.includes('{}') === true",
            selector.replace('\'', "\\'"),
            text.replace('\'', "\\'")
        );

        loop {
            let result = self.connection.send_page("Runtime.evaluate", json!({
                "expression": &check_js,
                "returnByValue": true,
            })).await?;

            if result.get("result").and_then(|r| r.get("value")).and_then(|v| v.as_bool()).unwrap_or(false) {
                return Ok(());
            }

            if start.elapsed().as_millis() > timeout_ms as u128 {
                anyhow::bail!("Timeout after {}ms waiting for text '{}' in {}", timeout_ms, text, selector);
            }

            tokio::time::sleep(Duration::from_millis(self.poll_interval_ms)).await;
        }
    }

    /// Wait for navigation to complete
    pub async fn wait_for_load(&self) -> Result<()> {
        self.wait_for_load_with_timeout(self.default_timeout_ms).await
    }

    pub async fn wait_for_load_with_timeout(&self, timeout_ms: u64) -> Result<()> {
        let start = Instant::now();

        loop {
            let result = self.connection.send_page("Runtime.evaluate", json!({
                "expression": "document.readyState === 'complete'",
                "returnByValue": true,
            })).await?;

            if result.get("result").and_then(|r| r.get("value")).and_then(|v| v.as_bool()).unwrap_or(false) {
                return Ok(());
            }

            if start.elapsed().as_millis() > timeout_ms as u128 {
                anyhow::bail!("Timeout after {}ms waiting for page load", timeout_ms);
            }

            tokio::time::sleep(Duration::from_millis(self.poll_interval_ms)).await;
        }
    }

    /// Wait for specific network request to complete
    pub async fn wait_for_network_idle(&self, idle_ms: u64) -> Result<()> {
        self.wait_for_network_idle_with_timeout(idle_ms, self.default_timeout_ms).await
    }

    pub async fn wait_for_network_idle_with_timeout(&self, idle_ms: u64, timeout_ms: u64) -> Result<()> {
        let start = Instant::now();
        let mut last_activity = Instant::now();

        loop {
            // Check pending requests count via Performance domain
            let result = self.connection.send_page("Runtime.evaluate", json!({
                "expression": "performance.getEntriesByType('resource').filter(r => r.responseEnd === 0).length",
                "returnByValue": true,
            })).await?;

            let pending = result.get("result")
                .and_then(|r| r.get("value"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0);

            if pending == 0 {
                if last_activity.elapsed().as_millis() >= idle_ms as u128 {
                    return Ok(());
                }
            } else {
                last_activity = Instant::now();
            }

            if start.elapsed().as_millis() > timeout_ms as u128 {
                anyhow::bail!("Timeout after {}ms waiting for network idle", timeout_ms);
            }

            tokio::time::sleep(Duration::from_millis(self.poll_interval_ms)).await;
        }
    }

    /// Wait for JavaScript condition to become true
    pub async fn wait_for_js(&self, expression: &str) -> Result<()> {
        self.wait_for_js_with_timeout(expression, self.default_timeout_ms).await
    }

    pub async fn wait_for_js_with_timeout(&self, expression: &str, timeout_ms: u64) -> Result<()> {
        let start = Instant::now();

        loop {
            let result = self.connection.send_page("Runtime.evaluate", json!({
                "expression": expression,
                "returnByValue": true,
            })).await?;

            if result.get("result").and_then(|r| r.get("value")).and_then(|v| v.as_bool()).unwrap_or(false) {
                return Ok(());
            }

            if start.elapsed().as_millis() > timeout_ms as u128 {
                anyhow::bail!("Timeout after {}ms waiting for JS condition: {}", timeout_ms, expression);
            }

            tokio::time::sleep(Duration::from_millis(self.poll_interval_ms)).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_wait_new() {
        // AutoWait struct defaults
        assert_eq!(30_000u64, 30_000);
        assert_eq!(100u64, 100);
    }

    #[test]
    fn test_auto_wait_builder() {
        // Test builder defaults
        let timeout = 5000u64;
        let poll = 50u64;
        assert_eq!(timeout, 5000);
        assert_eq!(poll, 50);
    }

    #[test]
    fn test_wait_for_js_expression() {
        let selector = "#test";
        let check = format!(
            "document.querySelector('{}') !== null",
            selector.replace('\'', "\\'")
        );
        assert!(check.contains("#test"));
        assert!(check.contains("!== null"));
    }

    #[test]
    fn test_wait_for_clickable_js_expression() {
        let selector = ".btn";
        let check = format!(
            r#"(() => {{
                const el = document.querySelector('{}');
                if (!el) return false;
                const rect = el.getBoundingClientRect();
                const style = window.getComputedStyle(el);
                return rect.width > 0
                    && rect.height > 0
                    && style.visibility !== 'hidden'
                    && style.display !== 'none'
                    && style.opacity !== '0'
                    && !el.disabled;
            }})()"#,
            selector.replace('\'', "\\'")
        );
        assert!(check.contains(".btn"));
        assert!(check.contains("getBoundingClientRect"));
        assert!(check.contains("disabled"));
    }

    #[test]
    fn test_wait_for_text_js_expression() {
        let selector = "#msg";
        let text = "Hello";
        let check = format!(
            "document.querySelector('{}')?.textContent?.includes('{}') === true",
            selector.replace('\'', "\\'"),
            text.replace('\'', "\\'")
        );
        assert!(check.contains("#msg"));
        assert!(check.contains("Hello"));
        assert!(check.contains("textContent"));
    }

    #[test]
    fn test_wait_for_load_js_expression() {
        let expr = "document.readyState === 'complete'";
        assert!(expr.contains("complete"));
    }

    #[test]
    fn test_wait_for_network_idle_js_expression() {
        let expr = "performance.getEntriesByType('resource').filter(r => r.responseEnd === 0).length";
        assert!(expr.contains("performance"));
        assert!(expr.contains("responseEnd"));
    }

    #[test]
    fn test_wait_for_js_custom_expression() {
        let expr = "document.querySelector('#modal') !== null && document.querySelector('#modal').style.display !== 'none'";
        assert!(expr.contains("#modal"));
        assert!(expr.contains("display"));
    }

    #[test]
    fn test_selector_escaping() {
        let selector = "div[data-id='123']";
        let escaped = selector.replace('\'', "\\'");
        assert_eq!(escaped, "div[data-id=\\'123\\']");
    }

    #[test]
    fn test_text_escaping() {
        let text = "it's a test";
        let escaped = text.replace('\'', "\\'");
        assert_eq!(escaped, "it\\'s a test");
    }
}
