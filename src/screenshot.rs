use anyhow::Result;
use serde::Serialize;
use serde_json::Value;
use std::sync::Arc;

use crate::browser::CdpConnection;

/// Screenshot capture with advanced options
pub struct ScreenshotCapture {
    connection: Arc<CdpConnection>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScreenshotOptions {
    /// Image format: "png" or "jpeg"
    pub format: String,
    /// JPEG quality (1-100), only for jpeg format
    pub quality: Option<u32>,
    /// Capture the full page (scrollable content)
    pub full_page: bool,
    /// Clip to a specific region
    pub clip: Option<ClipRegion>,
    /// Omit background (transparent for PNG)
    pub omit_background: bool,
    /// Device scale factor (for retina/HiDPI)
    pub scale: f64,
    /// Capture from surface (not just viewport)
    pub capture_beyond_viewport: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClipRegion {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub scale: f64,
}

impl Default for ScreenshotOptions {
    fn default() -> Self {
        Self {
            format: "png".to_string(),
            quality: None,
            full_page: false,
            clip: None,
            omit_background: false,
            scale: 1.0,
            capture_beyond_viewport: false,
        }
    }
}

impl ScreenshotOptions {
    pub fn png() -> Self {
        Self::default()
    }

    pub fn jpeg(quality: u32) -> Self {
        Self {
            format: "jpeg".to_string(),
            quality: Some(quality.clamp(1, 100)),
            ..Default::default()
        }
    }

    pub fn full_page(mut self) -> Self {
        self.full_page = true;
        self
    }

    pub fn clip(mut self, x: f64, y: f64, width: f64, height: f64) -> Self {
        self.clip = Some(ClipRegion {
            x,
            y,
            width,
            height,
            scale: 1.0,
        });
        self
    }

    pub fn omit_background(mut self) -> Self {
        self.omit_background = true;
        self
    }

    pub fn scale(mut self, scale: f64) -> Self {
        self.scale = scale;
        self
    }

    pub fn beyond_viewport(mut self) -> Self {
        self.capture_beyond_viewport = true;
        self
    }
}

impl ScreenshotCapture {
    pub fn new(connection: Arc<CdpConnection>) -> Self {
        Self { connection }
    }

    /// Take a screenshot and return raw bytes
    pub async fn capture(&self, options: &ScreenshotOptions) -> Result<Vec<u8>> {
        let mut params = serde_json::json!({
            "format": options.format,
            "captureBeyondViewport": options.capture_beyond_viewport,
            "skipScreenshot": false,
        });

        if let Some(quality) = options.quality {
            params["quality"] = serde_json::json!(quality);
        }

        if options.omit_background {
            params["omitBackground"] = serde_json::json!(true);
        }

        if let Some(ref clip) = options.clip {
            params["clip"] = serde_json::json!({
                "x": clip.x,
                "y": clip.y,
                "width": clip.width,
                "height": clip.height,
                "scale": clip.scale,
            });
        }

        if options.full_page {
            // Get page dimensions via JS
            let dims = self.connection.send_page("Runtime.evaluate", serde_json::json!({
                "expression": "JSON.stringify({width: document.documentElement.scrollWidth, height: document.documentElement.scrollHeight})",
                "returnByValue": true,
            })).await?;

            if let Some(text) = dims.get("result").and_then(|r| r.get("value")).and_then(|v| v.as_str()) {
                if let Ok(size) = serde_json::from_str::<Value>(text) {
                    let w = size["width"].as_f64().unwrap_or(1920.0);
                    let h = size["height"].as_f64().unwrap_or(1080.0);

                    // Set viewport to full page size
                    self.connection.send_page("Emulation.setDeviceMetricsOverride", serde_json::json!({
                        "width": w as u32,
                        "height": h as u32,
                        "deviceScaleFactor": options.scale,
                        "mobile": false,
                    })).await?;
                }
            }
        }

        if (options.scale - 1.0).abs() > 0.001 {
            // Set device scale factor for HiDPI
            let existing = self.connection.send_page("Runtime.evaluate", serde_json::json!({
                "expression": "JSON.stringify({w: window.innerWidth, h: window.innerHeight})",
                "returnByValue": true,
            })).await?;
            // Just set scale override without changing dimensions
            let _ = existing;
        }

        let result = self.connection.send_page("Page.captureScreenshot", params).await?;

        let data = result
            .get("data")
            .and_then(|d| d.as_str())
            .ok_or_else(|| anyhow::anyhow!("No screenshot data in response"))?;

        use base64::Engine;
        let bytes = base64::engine::general_purpose::STANDARD.decode(data)?;

        // Reset viewport if we changed it for full_page
        if options.full_page {
            let _ = self.connection.send_page("Emulation.clearDeviceMetricsOverride", serde_json::json!({})).await;
        }

        Ok(bytes)
    }

    /// Take a screenshot and save to file
    pub async fn save(&self, path: &str, options: &ScreenshotOptions) -> Result<()> {
        let bytes = self.capture(options).await?;
        tokio::fs::write(path, bytes).await?;
        Ok(())
    }

    /// Quick PNG screenshot
    pub async fn quick_save(&self, path: &str) -> Result<()> {
        self.save(path, &ScreenshotOptions::png()).await
    }

    /// Quick JPEG screenshot
    pub async fn jpeg_save(&self, path: &str, quality: u32) -> Result<()> {
        self.save(path, &ScreenshotOptions::jpeg(quality)).await
    }

    /// Screenshot a specific element by CSS selector
    pub async fn element_screenshot(&self, selector: &str, path: &str) -> Result<()> {
        // Get element bounding box
        let result = self.connection.send_page("Runtime.evaluate", serde_json::json!({
            "expression": format!(
                r#"(() => {{
                    const el = document.querySelector('{}');
                    if (!el) return null;
                    const rect = el.getBoundingClientRect();
                    return JSON.stringify({{
                        x: rect.x + window.scrollX,
                        y: rect.y + window.scrollY,
                        width: rect.width,
                        height: rect.height,
                    }});
                }})()"#,
                selector.replace('\'', "\\'")
            ),
            "returnByValue": true,
        })).await?;

        let rect_str = result
            .get("result")
            .and_then(|r| r.get("value"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Element not found: {}", selector))?;

        let rect: Value = serde_json::from_str(rect_str)?;
        let x = rect["x"].as_f64().unwrap_or(0.0);
        let y = rect["y"].as_f64().unwrap_or(0.0);
        let width = rect["width"].as_f64().unwrap_or(100.0);
        let height = rect["height"].as_f64().unwrap_or(100.0);

        let options = ScreenshotOptions {
            clip: Some(ClipRegion {
                x, y, width, height, scale: 1.0,
            }),
            ..Default::default()
        };

        self.save(path, &options).await
    }

    /// Take a base64 encoded screenshot (for API responses)
    pub async fn base64(&self, options: &ScreenshotOptions) -> Result<String> {
        let result = self.connection.send_page("Page.captureScreenshot", serde_json::json!({
            "format": options.format,
        })).await?;

        result
            .get("data")
            .and_then(|d| d.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("No screenshot data"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_screenshot_options_png() {
        let opts = ScreenshotOptions::png();
        assert_eq!(opts.format, "png");
        assert!(opts.quality.is_none());
        assert!(!opts.full_page);
    }

    #[test]
    fn test_screenshot_options_jpeg() {
        let opts = ScreenshotOptions::jpeg(80);
        assert_eq!(opts.format, "jpeg");
        assert_eq!(opts.quality, Some(80));
    }

    #[test]
    fn test_screenshot_options_jpeg_clamp() {
        let opts = ScreenshotOptions::jpeg(200);
        assert_eq!(opts.quality, Some(100));
        let opts = ScreenshotOptions::jpeg(0);
        assert_eq!(opts.quality, Some(1));
    }

    #[test]
    fn test_screenshot_options_builder() {
        let opts = ScreenshotOptions::png()
            .full_page()
            .clip(0.0, 0.0, 800.0, 600.0)
            .omit_background()
            .scale(2.0)
            .beyond_viewport();

        assert!(opts.full_page);
        assert!(opts.clip.is_some());
        assert!(opts.omit_background);
        assert!((opts.scale - 2.0).abs() < 0.001);
        assert!(opts.capture_beyond_viewport);
    }

    #[test]
    fn test_clip_region() {
        let clip = ClipRegion {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 50.0,
            scale: 1.5,
        };
        assert!((clip.x - 10.0).abs() < 0.001);
        assert!((clip.scale - 1.5).abs() < 0.001);
    }

    #[test]
    fn test_screenshot_options_serialize() {
        let opts = ScreenshotOptions::png();
        let json = serde_json::to_string(&opts).unwrap();
        assert!(json.contains("png"));
        assert!(json.contains("format"));
    }
}
