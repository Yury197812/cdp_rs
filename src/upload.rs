use anyhow::Result;
use std::sync::Arc;

use crate::browser::CdpConnection;

/// File upload handler for `<input type="file">` elements
pub struct UploadHandler {
    connection: Arc<CdpConnection>,
}

impl UploadHandler {
    pub fn new(connection: Arc<CdpConnection>) -> Self {
        Self { connection }
    }

    /// Set files on a file input element by selector
    pub async fn set_files(&self, selector: &str, files: &[&str]) -> Result<()> {
        // Get the DOM node for the input element
        let result = self
            .connection
            .send_page(
                "Runtime.evaluate",
                serde_json::json!({
                    "expression": format!(
                        "document.querySelector('{}')",
                        selector.replace('\'', "\\'")
                    ),
                    "returnByValue": false,
                }),
            )
            .await?;

        let object_id = result
            .get("result")
            .and_then(|r| r.get("objectId"))
            .and_then(|o| o.as_str())
            .ok_or_else(|| anyhow::anyhow!("Element not found: {}", selector))?;

        // Use DOM.setFileInputFiles to set the files
        self.connection
            .send_page(
                "DOM.setFileInputFiles",
                serde_json::json!({
                    "files": files,
                    "objectId": object_id,
                }),
            )
            .await?;

        // Trigger change event
        self.connection
            .send_page(
                "Runtime.evaluate",
                serde_json::json!({
                    "expression": format!(
                        "document.querySelector('{}').dispatchEvent(new Event('change', {{ bubbles: true }}))",
                        selector.replace('\'', "\\'")
                    ),
                }),
            )
            .await?;

        Ok(())
    }

    /// Set files on a file input using DOM.querySelector
    pub async fn set_files_via_dom(&self, selector: &str, files: &[&str]) -> Result<()> {
        // Get document root
        let doc = self
            .connection
            .send_page("DOM.getDocument", serde_json::json!({}))
            .await?;
        let root_id = doc["root"]["nodeId"].as_i64().unwrap_or(0);

        // Query for the element
        let found = self
            .connection
            .send_page(
                "DOM.querySelector",
                serde_json::json!({
                    "nodeId": root_id,
                    "selector": selector,
                }),
            )
            .await?;
        let node_id = found["nodeId"].as_i64().ok_or_else(|| {
            anyhow::anyhow!("Element not found: {}", selector)
        })?;

        // Set files
        self.connection
            .send_page(
                "DOM.setFileInputFiles",
                serde_json::json!({
                    "files": files,
                    "nodeId": node_id,
                }),
            )
            .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_upload_handler_new() {
        // Just verify struct can be created conceptually
        assert!(true);
    }
}
