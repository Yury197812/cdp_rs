use anyhow::Result;
use serde::Serialize;
use std::sync::Arc;

use crate::browser::CdpConnection;

/// PDF generation from browser pages
pub struct PdfGenerator {
    connection: Arc<CdpConnection>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PdfOptions {
    pub landscape: bool,
    #[serde(rename = "printBackground")]
    pub print_background: bool,
    pub scale: f64,
    #[serde(rename = "paperWidth")]
    pub paper_width: f64,
    #[serde(rename = "paperHeight")]
    pub paper_height: f64,
    #[serde(rename = "marginTop")]
    pub margin_top: f64,
    #[serde(rename = "marginBottom")]
    pub margin_bottom: f64,
    #[serde(rename = "marginLeft")]
    pub margin_left: f64,
    #[serde(rename = "marginRight")]
    pub margin_right: f64,
    #[serde(rename = "pageRanges")]
    pub page_ranges: Option<String>,
    #[serde(rename = "headerTemplate")]
    pub header_template: Option<String>,
    #[serde(rename = "footerTemplate")]
    pub footer_template: Option<String>,
    #[serde(rename = "preferCSSPageSize")]
    pub prefer_css_page_size: bool,
}

impl Default for PdfOptions {
    fn default() -> Self {
        Self {
            landscape: false,
            print_background: true,
            scale: 1.0,
            paper_width: 8.27,    // A4
            paper_height: 11.69,  // A4
            margin_top: 0.4,
            margin_bottom: 0.4,
            margin_left: 0.4,
            margin_right: 0.4,
            page_ranges: None,
            header_template: None,
            footer_template: None,
            prefer_css_page_size: true,
        }
    }
}

impl PdfOptions {
    pub fn a4() -> Self {
        Self::default()
    }

    pub fn letter() -> Self {
        Self {
            paper_width: 8.5,
            paper_height: 11.0,
            ..Default::default()
        }
    }

    pub fn landscape(mut self) -> Self {
        self.landscape = true;
        self
    }

    pub fn scale(mut self, scale: f64) -> Self {
        self.scale = scale;
        self
    }

    pub fn margins(mut self, top: f64, bottom: f64, left: f64, right: f64) -> Self {
        self.margin_top = top;
        self.margin_bottom = bottom;
        self.margin_left = left;
        self.margin_right = right;
        self
    }

    pub fn page_ranges(mut self, ranges: &str) -> Self {
        self.page_ranges = Some(ranges.to_string());
        self
    }

    pub fn header_template(mut self, template: &str) -> Self {
        self.header_template = Some(template.to_string());
        self
    }

    pub fn footer_template(mut self, template: &str) -> Self {
        self.footer_template = Some(template.to_string());
        self
    }
}

impl PdfGenerator {
    pub fn new(connection: Arc<CdpConnection>) -> Self {
        Self { connection }
    }

    /// Generate PDF from current page
    pub async fn generate(&self, options: &PdfOptions) -> Result<Vec<u8>> {
        let mut params = serde_json::json!({
            "landscape": options.landscape,
            "printBackground": options.print_background,
            "scale": options.scale,
            "paperWidth": options.paper_width,
            "paperHeight": options.paper_height,
            "marginTop": options.margin_top,
            "marginBottom": options.margin_bottom,
            "marginLeft": options.margin_left,
            "marginRight": options.margin_right,
            "preferCSSPageSize": options.prefer_css_page_size,
        });

        if let Some(ref ranges) = options.page_ranges {
            params["pageRanges"] = serde_json::Value::String(ranges.clone());
        }
        if let Some(ref header) = options.header_template {
            params["headerTemplate"] = serde_json::Value::String(header.clone());
        }
        if let Some(ref footer) = options.footer_template {
            params["footerTemplate"] = serde_json::Value::String(footer.clone());
        }

        let result = self.connection.send_page("Page.printToPDF", params).await?;

        let data = result
            .get("data")
            .and_then(|d| d.as_str())
            .ok_or_else(|| anyhow::anyhow!("No PDF data in response"))?;

        use base64::Engine;
        let bytes = base64::engine::general_purpose::STANDARD.decode(data)?;
        Ok(bytes)
    }

    /// Generate PDF and save to file
    pub async fn save(&self, path: &str, options: &PdfOptions) -> Result<()> {
        let bytes = self.generate(options).await?;
        tokio::fs::write(path, bytes).await?;
        Ok(())
    }

    /// Quick PDF generation with defaults
    pub async fn quick_save(&self, path: &str) -> Result<()> {
        self.save(path, &PdfOptions::default()).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pdf_options_default() {
        let opts = PdfOptions::default();
        assert!(!opts.landscape);
        assert!(opts.print_background);
        assert!((opts.scale - 1.0).abs() < 0.001);
        assert!((opts.paper_width - 8.27).abs() < 0.01);
        assert!((opts.paper_height - 11.69).abs() < 0.01);
    }

    #[test]
    fn test_pdf_options_a4() {
        let opts = PdfOptions::a4();
        assert!((opts.paper_width - 8.27).abs() < 0.01);
        assert!((opts.paper_height - 11.69).abs() < 0.01);
    }

    #[test]
    fn test_pdf_options_letter() {
        let opts = PdfOptions::letter();
        assert!((opts.paper_width - 8.5).abs() < 0.01);
        assert!((opts.paper_height - 11.0).abs() < 0.01);
    }

    #[test]
    fn test_pdf_options_builder() {
        let opts = PdfOptions::default()
            .landscape()
            .scale(1.5)
            .margins(1.0, 1.0, 0.5, 0.5)
            .page_ranges("1-3")
            .header_template("<div>Header</div>")
            .footer_template("<div>Page <span class='pageNumber'></span></div>");

        assert!(opts.landscape);
        assert!((opts.scale - 1.5).abs() < 0.001);
        assert!((opts.margin_top - 1.0).abs() < 0.001);
        assert_eq!(opts.page_ranges.as_deref(), Some("1-3"));
        assert!(opts.header_template.is_some());
        assert!(opts.footer_template.is_some());
    }

    #[test]
    fn test_pdf_options_serialize() {
        let opts = PdfOptions::default();
        let json = serde_json::to_string(&opts).unwrap();
        assert!(json.contains("paperWidth"));
        assert!(json.contains("landscape"));
    }
}
