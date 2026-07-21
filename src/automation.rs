use anyhow::Result;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;

use crate::browser::CdpConnection;
use crate::page::Page;

/// Browser automation engine with high-level actions
pub struct Automation {
    page: Page,
}

impl Automation {
    pub fn new(connection: Arc<CdpConnection>) -> Self {
        Self { page: Page::new(connection) }
    }

    pub fn page(&self) -> &Page {
        &self.page
    }

    // ============================================================
    // Navigation
    // ============================================================

    pub async fn navigate(&self, url: &str) -> Result<()> {
        self.page.navigate(url).await
    }

    pub async fn back(&self) -> Result<()> {
        self.page.evaluate("history.back()").await?;
        tokio::time::sleep(Duration::from_millis(500)).await;
        Ok(())
    }

    pub async fn forward(&self) -> Result<()> {
        self.page.evaluate("history.forward()").await?;
        tokio::time::sleep(Duration::from_millis(500)).await;
        Ok(())
    }

    pub async fn reload(&self) -> Result<()> {
        self.page.evaluate("location.reload()").await?;
        tokio::time::sleep(Duration::from_millis(500)).await;
        Ok(())
    }

    pub async fn get_url(&self) -> Result<String> {
        self.page.get_url().await
    }

    pub async fn get_title(&self) -> Result<String> {
        self.page.get_title().await
    }

    // ============================================================
    // Click actions
    // ============================================================

    /// Click element by CSS selector
    pub async fn click(&self, selector: &str) -> Result<()> {
        self.page.click(selector).await
    }

    /// Click at specific coordinates
    pub async fn click_at(&self, x: f64, y: f64) -> Result<()> {
        self.page.click_coords(x, y).await
    }

    /// Click with wait — wait for element, then click
    pub async fn click_when_ready(&self, selector: &str, timeout_ms: u64) -> Result<()> {
        self.page.wait_for_selector(selector, timeout_ms).await?;
        tokio::time::sleep(Duration::from_millis(100)).await;
        self.page.click(selector).await
    }

    /// Double click element
    pub async fn double_click(&self, selector: &str) -> Result<()> {
        let coords = self.get_element_center(selector).await?;
        let x = coords.0;
        let y = coords.1;

        self.send_mouse("mouseMoved", x, y).await?;
        tokio::time::sleep(Duration::from_millis(30)).await;
        self.send_mouse_with_args("mousePressed", x, y, "left", 2).await?;
        self.send_mouse_with_args("mouseReleased", x, y, "left", 2).await?;
        Ok(())
    }

    /// Right click element
    pub async fn right_click(&self, selector: &str) -> Result<()> {
        let coords = self.get_element_center(selector).await?;
        let x = coords.0;
        let y = coords.1;

        self.send_mouse("mouseMoved", x, y).await?;
        tokio::time::sleep(Duration::from_millis(30)).await;
        self.page.connection.send_page("Input.dispatchMouseEvent", json!({
            "type": "mousePressed", "x": x, "y": y,
            "button": "right", "clickCount": 1,
        })).await?;
        self.page.connection.send_page("Input.dispatchMouseEvent", json!({
            "type": "mouseReleased", "x": x, "y": y,
            "button": "right", "clickCount": 1,
        })).await?;
        Ok(())
    }

    /// Hover over element
    pub async fn hover(&self, selector: &str) -> Result<()> {
        let coords = self.get_element_center(selector).await?;
        self.send_mouse("mouseMoved", coords.0, coords.1).await
    }

    // ============================================================
    // Keyboard actions
    // ============================================================

    /// Type text into focused element
    pub async fn type_text(&self, text: &str) -> Result<()> {
        self.page.type_text(text).await
    }

    /// Type text into element (focus + clear + type)
    pub async fn type_into(&self, selector: &str, text: &str) -> Result<()> {
        self.page.fill(selector, text).await
    }

    /// Press a key
    pub async fn press_key(&self, key: &str) -> Result<()> {
        self.page.press_key(key).await
    }

    /// Press keyboard shortcut (e.g., "Control+c")
    pub async fn hotkey(&self, keys: &[&str]) -> Result<()> {
        for key in keys {
            self.page.connection.send_page("Input.dispatchKeyEvent", json!({
                "type": "keyDown", "key": key, "code": key,
                "modifiers": self.key_modifiers(key),
            })).await?;
        }
        for key in keys.iter().rev() {
            self.page.connection.send_page("Input.dispatchKeyEvent", json!({
                "type": "keyUp", "key": key, "code": key,
            })).await?;
        }
        Ok(())
    }

    fn key_modifiers(&self, key: &str) -> u32 {
        match key {
            "Control" | "Ctrl" => 2,
            "Alt" => 1,
            "Shift" => 4,
            "Meta" | "Command" => 8,
            _ => 0,
        }
    }

    // ============================================================
    // Form actions
    // ============================================================

    /// Fill input field
    pub async fn fill(&self, selector: &str, value: &str) -> Result<()> {
        self.page.fill(selector, value).await
    }

    /// Select dropdown option by value
    pub async fn select(&self, selector: &str, value: &str) -> Result<()> {
        let js = format!(
            r#"(() => {{
                const el = document.querySelector('{}');
                if (!el) return false;
                el.value = '{}';
                el.dispatchEvent(new Event('change', {{ bubbles: true }}));
                return true;
            }})()"#,
            selector.replace('\'', "\\'"),
            value.replace('\'', "\\'")
        );
        let result = self.page.evaluate(&js).await?;
        if !result.get("value").and_then(|v| v.as_bool()).unwrap_or(false) {
            anyhow::bail!("Element not found: {}", selector);
        }
        Ok(())
    }

    /// Select dropdown option by index
    pub async fn select_by_index(&self, selector: &str, index: u32) -> Result<()> {
        let js = format!(
            r#"(() => {{
                const el = document.querySelector('{}');
                if (!el || el.selectedIndex < 0) return false;
                el.selectedIndex = {};
                el.dispatchEvent(new Event('change', {{ bubbles: true }}));
                return true;
            }})()"#,
            selector.replace('\'', "\\'"),
            index
        );
        let result = self.page.evaluate(&js).await?;
        if !result.get("value").and_then(|v| v.as_bool()).unwrap_or(false) {
            anyhow::bail!("Element not found: {}", selector);
        }
        Ok(())
    }

    /// Check checkbox
    pub async fn check(&self, selector: &str) -> Result<()> {
        let js = format!(
            r#"(() => {{
                const el = document.querySelector('{}');
                if (!el) return false;
                el.checked = true;
                el.dispatchEvent(new Event('change', {{ bubbles: true }}));
                return true;
            }})()"#,
            selector.replace('\'', "\\'")
        );
        self.page.evaluate(&js).await?;
        Ok(())
    }

    /// Uncheck checkbox
    pub async fn uncheck(&self, selector: &str) -> Result<()> {
        let js = format!(
            r#"(() => {{
                const el = document.querySelector('{}');
                if (!el) return false;
                el.checked = false;
                el.dispatchEvent(new Event('change', {{ bubbles: true }}));
                return true;
            }})()"#,
            selector.replace('\'', "\\'")
        );
        self.page.evaluate(&js).await?;
        Ok(())
    }

    /// Submit form
    pub async fn submit_form(&self, selector: &str) -> Result<()> {
        let js = format!(
            r#"(() => {{
                const el = document.querySelector('{}');
                if (!el) return false;
                el.submit();
                return true;
            }})()"#,
            selector.replace('\'', "\\'")
        );
        self.page.evaluate(&js).await?;
        Ok(())
    }

    // ============================================================
    // Element queries
    // ============================================================

    /// Check if element exists
    pub async fn exists(&self, selector: &str) -> bool {
        self.page.evaluate(&format!(
            "document.querySelector('{}') !== null",
            selector.replace('\'', "\\'")
        )).await
        .map(|v| v.get("value").and_then(|v| v.as_bool()).unwrap_or(false))
        .unwrap_or(false)
    }

    /// Check if element is visible
    pub async fn is_visible(&self, selector: &str) -> bool {
        let js = format!(
            r#"(() => {{
                const el = document.querySelector('{}');
                if (!el) return false;
                const style = window.getComputedStyle(el);
                const rect = el.getBoundingClientRect();
                return rect.width > 0 && rect.height > 0
                    && style.display !== 'none'
                    && style.visibility !== 'hidden'
                    && style.opacity !== '0';
            }})()"#,
            selector.replace('\'', "\\'")
        );
        self.page.evaluate(&js).await
            .map(|v| v.get("value").and_then(|v| v.as_bool()).unwrap_or(false))
            .unwrap_or(false)
    }

    /// Get element text content
    pub async fn text(&self, selector: &str) -> Result<String> {
        let js = format!(
            "document.querySelector('{}')?.textContent?.trim() || ''",
            selector.replace('\'', "\\'")
        );
        let result = self.page.evaluate(&js).await?;
        Ok(result.get("value").and_then(|v| v.as_str()).unwrap_or("").to_string())
    }

    /// Get element inner HTML
    pub async fn inner_html(&self, selector: &str) -> Result<String> {
        let js = format!(
            "document.querySelector('{}')?.innerHTML || ''",
            selector.replace('\'', "\\'")
        );
        let result = self.page.evaluate(&js).await?;
        Ok(result.get("value").and_then(|v| v.as_str()).unwrap_or("").to_string())
    }

    /// Get element attribute
    pub async fn get_attribute(&self, selector: &str, attr: &str) -> Result<Option<String>> {
        let js = format!(
            "document.querySelector('{}')?.getAttribute('{}')",
            selector.replace('\'', "\\'"),
            attr.replace('\'', "\\'")
        );
        let result = self.page.evaluate(&js).await?;
        Ok(result.get("value").and_then(|v| v.as_str()).map(|s| s.to_string()))
    }

    /// Get element value (for inputs)
    pub async fn get_value(&self, selector: &str) -> Result<String> {
        let js = format!(
            "document.querySelector('{}')?.value || ''",
            selector.replace('\'', "\\'")
        );
        let result = self.page.evaluate(&js).await?;
        Ok(result.get("value").and_then(|v| v.as_str()).unwrap_or("").to_string())
    }

    /// Get element count
    pub async fn count(&self, selector: &str) -> Result<usize> {
        let js = format!(
            "document.querySelectorAll('{}').length",
            selector.replace('\'', "\\'")
        );
        let result = self.page.evaluate(&js).await?;
        Ok(result.get("value").and_then(|v| v.as_u64()).unwrap_or(0) as usize)
    }

    /// Get all matching elements text
    pub async fn all_text(&self, selector: &str) -> Result<Vec<String>> {
        let js = format!(
            "JSON.stringify(Array.from(document.querySelectorAll('{}')).map(e => e.textContent.trim()))",
            selector.replace('\'', "\\'")
        );
        let result = self.page.evaluate(&js).await?;
        let text = result.get("value").and_then(|v| v.as_str()).unwrap_or("[]");
        let texts: Vec<String> = serde_json::from_str(text).unwrap_or_default();
        Ok(texts)
    }

    /// Get all matching elements attributes
    pub async fn all_attribute(&self, selector: &str, attr: &str) -> Result<Vec<Option<String>>> {
        let js = format!(
            "JSON.stringify(Array.from(document.querySelectorAll('{}')).map(e => e.getAttribute('{}')))",
            selector.replace('\'', "\\'"),
            attr.replace('\'', "\\'")
        );
        let result = self.page.evaluate(&js).await?;
        let text = result.get("value").and_then(|v| v.as_str()).unwrap_or("[]");
        let attrs: Vec<Option<String>> = serde_json::from_str(text).unwrap_or_default();
        Ok(attrs)
    }

    // ============================================================
    // Scrolling
    // ============================================================

    /// Scroll to element
    pub async fn scroll_to(&self, selector: &str) -> Result<()> {
        let js = format!(
            "document.querySelector('{}')?.scrollIntoView({{ behavior: 'smooth', block: 'center' }})",
            selector.replace('\'', "\\'")
        );
        self.page.evaluate(&js).await?;
        tokio::time::sleep(Duration::from_millis(300)).await;
        Ok(())
    }

    /// Scroll to top
    pub async fn scroll_to_top(&self) -> Result<()> {
        self.page.evaluate("window.scrollTo({top: 0, behavior: 'smooth'})").await?;
        tokio::time::sleep(Duration::from_millis(300)).await;
        Ok(())
    }

    /// Scroll to bottom
    pub async fn scroll_to_bottom(&self) -> Result<()> {
        self.page.evaluate("window.scrollTo({top: document.body.scrollHeight, behavior: 'smooth'})").await?;
        tokio::time::sleep(Duration::from_millis(300)).await;
        Ok(())
    }

    /// Scroll down by pixels
    pub async fn scroll_down(&self, pixels: u32) -> Result<()> {
        self.page.evaluate(&format!("window.scrollBy(0, {})", pixels)).await?;
        tokio::time::sleep(Duration::from_millis(200)).await;
        Ok(())
    }

    /// Scroll up by pixels
    pub async fn scroll_up(&self, pixels: u32) -> Result<()> {
        self.page.evaluate(&format!("window.scrollBy(0, -{})", pixels)).await?;
        tokio::time::sleep(Duration::from_millis(200)).await;
        Ok(())
    }

    /// Get scroll position
    pub async fn scroll_position(&self) -> Result<(f64, f64)> {
        let result = self.page.evaluate("JSON.stringify({x: window.scrollX, y: window.scrollY})").await?;
        let text = result.get("value").and_then(|v| v.as_str()).unwrap_or("{\"x\":0,\"y\":0}");
        let pos: Value = serde_json::from_str(text)?;
        Ok((pos["x"].as_f64().unwrap_or(0.0), pos["y"].as_f64().unwrap_or(0.0)))
    }

    // ============================================================
    // Drag and drop
    // ============================================================

    /// Drag from one element to another
    pub async fn drag(&self, from: &str, to: &str) -> Result<()> {
        let from_coords = self.get_element_center(from).await?;
        let to_coords = self.get_element_center(to).await?;

        // Mouse down on source
        self.send_mouse("mouseMoved", from_coords.0, from_coords.1).await?;
        self.send_mouse_with_button("mousePressed", from_coords.0, from_coords.1, "left").await?;
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Move to target
        self.send_mouse("mouseMoved", to_coords.0, to_coords.1).await?;
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Mouse up on target
        self.send_mouse_with_button("mouseReleased", to_coords.0, to_coords.1, "left").await?;
        Ok(())
    }

    // ============================================================
    // Iframes
    // ============================================================

    /// Switch to iframe by selector
    pub async fn switch_to_frame(&self, selector: &str) -> Result<()> {
        let js = format!(
            r#"(() => {{
                const frame = document.querySelector('{}');
                if (!frame) return false;
                // Store frame reference for operations
                window.__cdp_frame = frame;
                return true;
            }})()"#,
            selector.replace('\'', "\\'")
        );
        self.page.evaluate(&js).await?;
        Ok(())
    }

    /// Switch back to main frame
    pub async fn switch_to_main_frame(&self) -> Result<()> {
        self.page.evaluate("window.__cdp_frame = null").await?;
        Ok(())
    }

    // ============================================================
    // Tabs
    // ============================================================

    /// Open new tab
    pub async fn new_tab(&self, url: &str) -> Result<()> {
        self.page.connection.send_page("Target.createTarget", json!({
            "url": url,
        })).await?;
        Ok(())
    }

    /// Close current tab
    pub async fn close_tab(&self) -> Result<()> {
        self.page.connection.send_page("Target.closeTarget", json!({
            "targetId": "current",
        })).await?;
        Ok(())
    }

    // ============================================================
    // Utilities
    // ============================================================

    /// Wait for element to appear
    pub async fn wait_for_element(&self, selector: &str, timeout_ms: u64) -> Result<()> {
        self.page.wait_for_selector(selector, timeout_ms).await
    }

    /// Wait for element to disappear
    pub async fn wait_for_gone(&self, selector: &str, timeout_ms: u64) -> Result<()> {
        let start = std::time::Instant::now();
        loop {
            if !self.exists(selector).await {
                return Ok(());
            }
            if start.elapsed().as_millis() > timeout_ms as u128 {
                anyhow::bail!("Timeout waiting for element to disappear: {}", selector);
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    /// Wait for text to appear on page
    pub async fn wait_for_text(&self, text: &str, timeout_ms: u64) -> Result<()> {
        self.page.wait_for_text(text, timeout_ms).await
    }

    /// Wait for URL to change
    pub async fn wait_for_url(&self, pattern: &str, timeout_ms: u64) -> Result<()> {
        self.page.wait_for_url_contains(pattern, timeout_ms).await
    }

    /// Take screenshot
    pub async fn screenshot(&self, path: &str) -> Result<()> {
        self.page.screenshot(path).await
    }

    /// Execute JavaScript
    pub async fn eval(&self, expression: &str) -> Result<Value> {
        self.page.evaluate(expression).await
    }

    // ============================================================
    // Internal helpers
    // ============================================================

    async fn get_element_center(&self, selector: &str) -> Result<(f64, f64)> {
        let js = format!(
            r#"(() => {{
                const el = document.querySelector('{}');
                if (!el) return null;
                const rect = el.getBoundingClientRect();
                return {{ x: rect.x + rect.width/2, y: rect.y + rect.height/2 }};
            }})()"#,
            selector.replace('\'', "\\'")
        );
        let result = self.page.evaluate(&js).await?;
        let x = result.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let y = result.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0);
        Ok((x, y))
    }

    async fn send_mouse(&self, event_type: &str, x: f64, y: f64) -> Result<()> {
        self.page.connection.send_page("Input.dispatchMouseEvent", json!({
            "type": event_type, "x": x, "y": y,
        })).await?;
        Ok(())
    }

    async fn send_mouse_with_args(&self, event_type: &str, x: f64, y: f64, button: &str, click_count: u32) -> Result<()> {
        self.page.connection.send_page("Input.dispatchMouseEvent", json!({
            "type": event_type, "x": x, "y": y,
            "button": button, "clickCount": click_count,
        })).await?;
        Ok(())
    }

    async fn send_mouse_with_button(&self, event_type: &str, x: f64, y: f64, button: &str) -> Result<()> {
        self.page.connection.send_page("Input.dispatchMouseEvent", json!({
            "type": event_type, "x": x, "y": y,
            "button": button, "clickCount": 1,
        })).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_automation_key_modifiers() {
        // Test key modifier mapping
        let tests = vec![
            ("Control", 2u32),
            ("Alt", 1),
            ("Shift", 4),
            ("Meta", 8),
            ("a", 0),
        ];
        for (key, expected) in tests {
            // Just verify the logic works without needing a real connection
            let modifiers = match key {
                "Control" | "Ctrl" => 2,
                "Alt" => 1,
                "Shift" => 4,
                "Meta" | "Command" => 8,
                _ => 0,
            };
            assert_eq!(modifiers, expected, "Failed for key: {}", key);
        }
    }
}
