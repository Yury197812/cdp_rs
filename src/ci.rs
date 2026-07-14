use anyhow::Result;
use serde::{Deserialize, Serialize};

pub struct GitHubCI {
    token: String,
    client: reqwest::Client,
}

impl GitHubCI {
    pub fn new(token: &str) -> Self {
        Self {
            token: token.to_string(),
            client: reqwest::Client::new(),
        }
    }

    pub async fn create_check_run(&self, repo: &str, sha: &str, name: &str, status: &str) -> Result<()> {
        let url = format!("https://api.github.com/repos/{repo}/check-runs");
        self.client.post(&url)
            .header("Authorization", format!("token {}", self.token))
            .header("Accept", "application/vnd.github.v3+json")
            .json(&serde_json::json!({
                "name": name,
                "head_sha": sha,
                "status": status,
            }))
            .send()
            .await?;
        Ok(())
    }

    pub async fn create_pr_comment(&self, repo: &str, pr: u32, body: &str) -> Result<()> {
        let url = format!("https://api.github.com/repos/{repo}/issues/{pr}/comments");
        self.client.post(&url)
            .header("Authorization", format!("token {}", self.token))
            .header("Accept", "application/vnd.github.v3+json")
            .json(&serde_json::json!({ "body": body }))
            .send()
            .await?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CIConfig {
    pub name: String,
    pub shards: usize,
    pub timeout_minutes: u32,
    pub browsers: Vec<String>,
}
