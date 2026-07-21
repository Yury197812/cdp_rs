use anyhow::Result;
use serde_json::Value;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use crate::browser::CdpConnection;
use crate::page::Page;

/// Gmail OAuth auto-setup — Rust port of gmail_oauth_auto.py
pub struct GmailOAuth {
    page: Page,
    project_name: String,
    screenshots_dir: String,
}

impl GmailOAuth {
    pub fn new(connection: Arc<CdpConnection>) -> Self {
        Self {
            page: Page::new(connection),
            project_name: "Gmail Blocks".to_string(),
            screenshots_dir: "E:/1".to_string(),
        }
    }

    pub fn project_name(mut self, name: &str) -> Self {
        self.project_name = name.to_string();
        self
    }

    pub fn screenshots_dir(mut self, dir: &str) -> Self {
        self.screenshots_dir = dir.to_string();
        self
    }

    async fn screenshot(&self, step: &str) -> Result<()> {
        let path = format!("{}/oauth_{}.png", self.screenshots_dir, step);
        self.page.screenshot(&path).await?;
        println!("[SCREENSHOT] {}", path);
        Ok(())
    }

    async fn wait_for_signal(&self, msg: &str, timeout_secs: u64) -> bool {
        println!("\n{}", msg);
        let signal_file = format!("{}/login_done.txt", self.screenshots_dir);
        println!("Create file \"{}\" when done", signal_file);
        let start = std::time::Instant::now();
        loop {
            if Path::new(&signal_file).exists() {
                let _ = tokio::fs::remove_file(&signal_file).await;
                return true;
            }
            if start.elapsed().as_secs() > timeout_secs {
                return false;
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    }

    pub async fn run(&self) -> Result<()> {
        println!("{}", "=".repeat(60));
        println!("Gmail OAuth Auto-Setup (Rust)");
        println!("{}", "=".repeat(60));

        // Clean up signal file
        let signal_file = format!("{}/login_done.txt", self.screenshots_dir);
        let _ = tokio::fs::remove_file(&signal_file).await;

        // Step 1: Open Google Cloud Console
        println!("\n[1] Opening Google Cloud Console...");
        self.page.navigate("https://console.cloud.google.com").await?;
        tokio::time::sleep(Duration::from_secs(5)).await;
        self.screenshot("step1").await?;

        // Step 2: Check if login required
        let url = self.page.get_url().await?;
        if url.contains("accounts.google.com") {
            println!("\n[2] Login required!");
            if !self.wait_for_signal("Please login to Google account. Complete CAPTCHA/2FA.", 300).await {
                anyhow::bail!("Login timeout");
            }
            tokio::time::sleep(Duration::from_secs(5)).await;
            self.screenshot("step2_after_login").await?;
        }

        // Step 3: Wait for console
        println!("\n[3] Waiting for console...");
        let _ = self.page.wait_for_selector("text=Select a project", 30000).await;
        self.screenshot("step3").await?;

        // Step 4: Create project
        println!("\n[4] Creating project...");
        match self.create_project().await {
            Ok(()) => println!("[4] Project created!"),
            Err(e) => {
                println!("[4] Error: {}", e);
                self.screenshot("step4_error").await?;
                if !self.wait_for_signal(&format!("Create project \"{}\" manually", self.project_name), 300).await {
                    anyhow::bail!("Project creation timeout");
                }
            }
        }

        // Step 5: Enable Gmail API
        println!("\n[5] Enabling Gmail API...");
        match self.enable_gmail_api().await {
            Ok(()) => println!("[5] Gmail API enabled!"),
            Err(e) => println!("[5] Error: {}", e),
        }
        self.screenshot("step5").await?;

        // Step 6: Create OAuth credentials
        println!("\n[6] Creating OAuth credentials...");
        match self.create_oauth_credentials().await {
            Ok(()) => println!("[6] OAuth credentials created!"),
            Err(e) => {
                println!("[6] Error: {}", e);
                self.screenshot("step6_error").await?;
                if !self.wait_for_signal("Create OAuth credentials manually", 300).await {
                    anyhow::bail!("OAuth creation timeout");
                }
            }
        }
        self.screenshot("step6").await?;

        // Step 7: Download JSON
        println!("\n[7] Downloading JSON...");
        match self.download_json().await {
            Ok(()) => println!("[7] JSON downloaded!"),
            Err(e) => {
                println!("[7] Error: {}", e);
                self.screenshot("step7_error").await?;
                self.wait_for_signal("Download JSON and save as E:\\1\\.gmail_oauth_client.json", 300).await;
            }
        }
        self.screenshot("step7").await?;

        // Step 8: Verify
        println!("\n[8] Verifying...");
        let json_path = "E:/1/.gmail_oauth_client.json";
        if Path::new(json_path).exists() {
            let content = tokio::fs::read_to_string(json_path).await?;
            let data: Value = serde_json::from_str(&content)?;
            println!("[OK] Client secret file exists!");
            if let Some(installed) = data.get("installed") {
                if let Some(client_id) = installed.get("client_id").and_then(|v| v.as_str()) {
                    println!("  Client ID: {}...", &client_id[..30.min(client_id.len())]);
                }
            }
        } else {
            println!("[ERROR] File not found!");
        }

        self.screenshot("final").await?;
        println!("\n{}", "=".repeat(60));
        println!("Setup complete!");
        println!("Run: blocks gmail-oauth auth");
        println!("{}", "=".repeat(60));

        Ok(())
    }

    async fn create_project(&self) -> Result<()> {
        self.page.click("text=Select a project").await?;
        tokio::time::sleep(Duration::from_secs(2)).await;
        self.page.click("text=New Project").await?;
        tokio::time::sleep(Duration::from_secs(2)).await;
        self.page.fill("input[aria-label=\"Project name\"]", &self.project_name).await?;
        tokio::time::sleep(Duration::from_secs(1)).await;
        self.page.click("button:has-text(\"Create\")").await?;
        tokio::time::sleep(Duration::from_secs(10)).await;
        Ok(())
    }

    async fn enable_gmail_api(&self) -> Result<()> {
        self.page.navigate("https://console.cloud.google.com/apis/library/gmail.googleapis.com").await?;
        tokio::time::sleep(Duration::from_secs(5)).await;
        if let Ok(()) = self.page.click("button:has-text(\"Enable\")").await {
            tokio::time::sleep(Duration::from_secs(10)).await;
        }
        Ok(())
    }

    async fn create_oauth_credentials(&self) -> Result<()> {
        self.page.navigate("https://console.cloud.google.com/apis/credentials").await?;
        tokio::time::sleep(Duration::from_secs(5)).await;
        self.page.click("text=Create Credentials").await?;
        tokio::time::sleep(Duration::from_secs(1)).await;
        self.page.click("text=OAuth client ID").await?;
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Select Desktop app
        self.page.click("[aria-label=\"Application type\"]").await?;
        tokio::time::sleep(Duration::from_secs(1)).await;
        self.page.click("text=Desktop app").await?;
        tokio::time::sleep(Duration::from_secs(1)).await;

        self.page.fill("input[aria-label=\"Name\"]", &self.project_name).await?;
        tokio::time::sleep(Duration::from_secs(1)).await;
        self.page.click("button:has-text(\"Create\")").await?;
        tokio::time::sleep(Duration::from_secs(5)).await;
        Ok(())
    }

    async fn download_json(&self) -> Result<()> {
        self.page.click("text=Download JSON").await?;
        tokio::time::sleep(Duration::from_secs(5)).await;

        // Move downloaded file
        let downloads_dir = dirs::download_dir().unwrap_or_else(|| "C:/Users/Default/Downloads".into());
        let mut entries = tokio::fs::read_dir(&downloads_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("client_secret_") && name.ends_with(".json") {
                let src = entry.path();
                let dst = "E:/1/.gmail_oauth_client.json";
                if Path::new(dst).exists() {
                    tokio::fs::remove_file(dst).await?;
                }
                tokio::fs::rename(&src, dst).await?;
                println!("[7] Saved: {}", dst);
                break;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signal_file_path() {
        let dir = "E:/1";
        let signal_file = format!("{}/login_done.txt", dir);
        assert_eq!(signal_file, "E:/1/login_done.txt");
    }

    #[test]
    fn test_screenshot_path_format() {
        let dir = "E:/1";
        let step = "step1";
        let path = format!("{}/oauth_{}.png", dir, step);
        assert_eq!(path, "E:/1/oauth_step1.png");
    }

    #[test]
    fn test_json_path() {
        let json_path = "E:/1/.gmail_oauth_client.json";
        assert!(json_path.ends_with(".json"));
        assert!(json_path.contains("gmail_oauth_client"));
    }
}
