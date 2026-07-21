use anyhow::Result;
use serde::{Deserialize, Serialize};

/// GitHub Actions CI integration for browser test reporting
pub struct GitHubCI {
    token: String,
    client: reqwest::Client,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CIConfig {
    pub name: String,
    pub shards: usize,
    pub timeout_minutes: u32,
    pub browsers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckRunResult {
    pub id: u64,
    pub html_url: String,
    pub status: String,
    pub conclusion: Option<String>,
}

impl GitHubCI {
    pub fn new(token: &str) -> Self {
        Self {
            token: token.to_string(),
            client: reqwest::Client::new(),
        }
    }

    fn headers(&self) -> Vec<(&str, &str)> {
        vec![
            ("Authorization", &self.token),
            ("Accept", "application/vnd.github.v3+json"),
        ]
    }

    /// Create a check run and return its ID for later updates
    pub async fn create_check_run(
        &self,
        repo: &str,
        sha: &str,
        name: &str,
        status: &str,
    ) -> Result<CheckRunResult> {
        let url = format!("https://api.github.com/repos/{repo}/check-runs");
        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("token {}", self.token))
            .header("Accept", "application/vnd.github.v3+json")
            .json(&serde_json::json!({
                "name": name,
                "head_sha": sha,
                "status": status,
            }))
            .send()
            .await?;

        let body: serde_json::Value = resp.json().await?;
        Ok(CheckRunResult {
            id: body["id"].as_u64().unwrap_or(0),
            html_url: body["html_url"].as_str().unwrap_or("").to_string(),
            status: body["status"].as_str().unwrap_or("unknown").to_string(),
            conclusion: body["conclusion"].as_str().map(|s| s.to_string()),
        })
    }

    /// Update a check run with results
    pub async fn update_check_run(
        &self,
        repo: &str,
        check_run_id: u64,
        status: &str,
        conclusion: Option<&str>,
        output: Option<&serde_json::Value>,
    ) -> Result<()> {
        let url = format!(
            "https://api.github.com/repos/{repo}/check-runs/{check_run_id}"
        );
        let mut body = serde_json::json!({
            "status": status,
        });
        if let Some(c) = conclusion {
            body["conclusion"] = serde_json::Value::String(c.to_string());
        }
        if let Some(o) = output {
            body["output"] = o.clone();
        }

        self.client
            .patch(&url)
            .header("Authorization", format!("token {}", self.token))
            .header("Accept", "application/vnd.github.v3+json")
            .json(&body)
            .send()
            .await?;
        Ok(())
    }

    /// Post a comment on a PR/issue
    pub async fn create_pr_comment(
        &self,
        repo: &str,
        pr: u32,
        body: &str,
    ) -> Result<()> {
        let url = format!("https://api.github.com/repos/{repo}/issues/{pr}/comments");
        self.client
            .post(&url)
            .header("Authorization", format!("token {}", self.token))
            .header("Accept", "application/vnd.github.v3+json")
            .json(&serde_json::json!({ "body": body }))
            .send()
            .await?;
        Ok(())
    }

    /// Delete a comment
    pub async fn delete_comment(&self, repo: &str, comment_id: u64) -> Result<()> {
        let url = format!(
            "https://api.github.com/repos/{repo}/issues/comments/{comment_id}"
        );
        self.client
            .delete(&url)
            .header("Authorization", format!("token {}", self.token))
            .send()
            .await?;
        Ok(())
    }

    /// List open PRs
    pub async fn list_open_prs(&self, repo: &str) -> Result<Vec<PRInfo>> {
        let url = format!("https://api.github.com/repos/{repo}/pulls?state=open");
        let resp = self
            .client
            .get(&url)
            .header("Authorization", format!("token {}", self.token))
            .header("Accept", "application/vnd.github.v3+json")
            .send()
            .await?;

        let items: Vec<serde_json::Value> = resp.json().await?;
        let prs = items
            .iter()
            .map(|v| PRInfo {
                number: v["number"].as_u64().unwrap_or(0) as u32,
                title: v["title"].as_str().unwrap_or("").to_string(),
                head_sha: v["head"]["sha"].as_str().unwrap_or("").to_string(),
                html_url: v["html_url"].as_str().unwrap_or("").to_string(),
            })
            .collect();
        Ok(prs)
    }

    /// Create a test report comment on a PR
    pub async fn post_test_report(
        &self,
        repo: &str,
        pr: u32,
        report: &crate::test_runner::TestReport,
    ) -> Result<()> {
        let status_emoji = if report.failed == 0 { "✅" } else { "❌" };
        let body = format!(
            "## {} CDP-RS Test Results\n\n\
            | Metric | Value |\n\
            |--------|-------|\n\
            | Passed | {} |\n\
            | Failed | {} |\n\
            | Skipped | {} |\n\
            | Total | {} |\n\
            | Duration | {}ms |\n\n\
            <details><summary>Detailed Results</summary>\n\n\
            | Test | Status | Duration | Attempts |\n\
            |------|--------|----------|----------|\n{}\n\
            </details>",
            status_emoji,
            report.passed,
            report.failed,
            report.skipped,
            report.total,
            report.total_duration_ms,
            report
                .tests
                .iter()
                .map(|t| {
                    let icon = match t.status {
                        crate::test_runner::TestStatus::Passed => "✅",
                        crate::test_runner::TestStatus::Failed => "❌",
                        crate::test_runner::TestStatus::Skipped => "⏭️",
                    };
                    format!(
                        "| {} | {} {} | {}ms | {}/{} |",
                        t.name,
                        icon,
                        format!("{:?}", t.status),
                        t.duration_ms,
                        t.attempt,
                        t.attempt + if t.error.is_some() { 0 } else { 0 },
                    )
                })
                .collect::<Vec<_>>()
                .join("\n"),
        );
        self.create_pr_comment(repo, pr, &body).await
    }
}

#[derive(Debug, Clone)]
pub struct PRInfo {
    pub number: u32,
    pub title: String,
    pub head_sha: String,
    pub html_url: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ci_config_serialize() {
        let config = CIConfig {
            name: "browser-tests".to_string(),
            shards: 4,
            timeout_minutes: 30,
            browsers: vec!["chrome".to_string(), "firefox".to_string()],
        };
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("browser-tests"));
        assert!(json.contains("chrome"));
    }

    #[test]
    fn test_ci_config_deserialize() {
        let json = r#"{"name":"test","shards":2,"timeout_minutes":15,"browsers":["safari"]}"#;
        let config: CIConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.name, "test");
        assert_eq!(config.shards, 2);
        assert_eq!(config.browsers, vec!["safari"]);
    }

    #[test]
    fn test_check_run_result() {
        let result = CheckRunResult {
            id: 123,
            html_url: "https://github.com/test".to_string(),
            status: "completed".to_string(),
            conclusion: Some("success".to_string()),
        };
        assert_eq!(result.id, 123);
        assert!(result.conclusion.is_some());
    }

    #[test]
    fn test_pr_info() {
        let pr = PRInfo {
            number: 42,
            title: "Add feature".to_string(),
            head_sha: "abc123".to_string(),
            html_url: "https://github.com/test/pull/42".to_string(),
        };
        assert_eq!(pr.number, 42);
    }
}
