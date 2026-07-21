use anyhow::Result;
use serde_json::Value;
use std::sync::Arc;

use crate::browser::CdpConnection;

/// Browser dialog handler (alert, confirm, prompt, beforeunload)
pub struct DialogHandler {
    connection: Arc<CdpConnection>,
}

#[derive(Debug, Clone)]
pub struct DialogEvent {
    pub dialog_type: String,
    pub message: String,
    pub url: String,
    pub has_browser_handler: bool,
    pub default_prompt: Option<String>,
}

impl DialogHandler {
    pub fn new(connection: Arc<CdpConnection>) -> Self {
        Self { connection }
    }

    /// Enable dialog handling
    pub async fn enable(&self) -> Result<()> {
        self.connection
            .send_page("Page.enable", serde_json::json!({}))
            .await?;
        Ok(())
    }

    /// Handle the next dialog by accepting it
    pub async fn handle_next_accept(&self) -> Result<()> {
        let mut rx = self.connection.subscribe();
        while let Ok(event) = rx.recv().await {
            if event.method == "Page.javascriptDialogOpening" {
                self.connection
                    .send_page(
                        "Page.handleJavaScriptDialog",
                        serde_json::json!({ "accept": true }),
                    )
                    .await?;
                return Ok(());
            }
        }
        Ok(())
    }

    /// Handle the next dialog by dismissing it
    pub async fn handle_next_dismiss(&self) -> Result<()> {
        let mut rx = self.connection.subscribe();
        while let Ok(event) = rx.recv().await {
            if event.method == "Page.javascriptDialogOpening" {
                self.connection
                    .send_page(
                        "Page.handleJavaScriptDialog",
                        serde_json::json!({ "accept": false }),
                    )
                    .await?;
                return Ok(());
            }
        }
        Ok(())
    }

    /// Handle the next prompt dialog with a response
    pub async fn handle_next_prompt(&self, response: &str) -> Result<()> {
        let mut rx = self.connection.subscribe();
        while let Ok(event) = rx.recv().await {
            if event.method == "Page.javascriptDialogOpening" {
                self.connection
                    .send_page(
                        "Page.handleJavaScriptDialog",
                        serde_json::json!({
                            "accept": true,
                            "promptText": response,
                        }),
                    )
                    .await?;
                return Ok(());
            }
        }
        Ok(())
    }

    /// Parse a dialog event from raw CDP event
    pub fn parse_event(params: &Value) -> Option<DialogEvent> {
        Some(DialogEvent {
            dialog_type: params["type"].as_str()?.to_string(),
            message: params["message"].as_str().unwrap_or("").to_string(),
            url: params["url"].as_str().unwrap_or("").to_string(),
            has_browser_handler: params["hasBrowserHandler"].as_bool().unwrap_or(false),
            default_prompt: params["defaultPrompt"].as_str().map(|s| s.to_string()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dialog_event_parse() {
        let params = serde_json::json!({
            "type": "alert",
            "message": "Hello!",
            "url": "https://example.com",
            "hasBrowserHandler": false,
        });
        let event = DialogHandler::parse_event(&params).unwrap();
        assert_eq!(event.dialog_type, "alert");
        assert_eq!(event.message, "Hello!");
        assert!(!event.has_browser_handler);
    }

    #[test]
    fn test_dialog_event_prompt() {
        let params = serde_json::json!({
            "type": "prompt",
            "message": "Enter name:",
            "url": "https://example.com",
            "defaultPrompt": "default",
        });
        let event = DialogHandler::parse_event(&params).unwrap();
        assert_eq!(event.dialog_type, "prompt");
        assert_eq!(event.default_prompt.as_deref(), Some("default"));
    }

    #[test]
    fn test_dialog_event_confirm() {
        let params = serde_json::json!({
            "type": "confirm",
            "message": "Are you sure?",
            "url": "",
        });
        let event = DialogHandler::parse_event(&params).unwrap();
        assert_eq!(event.dialog_type, "confirm");
        assert!(event.url.is_empty());
    }
}
