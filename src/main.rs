mod browser;
mod test_runner;
mod ci;

use browser::BrowserManager;
use test_runner::TestRunner;
use ci::GitHubCI;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("CDP-RS: Browser Automation Framework (Rust)");
    println!("Based on critical analysis of 78+ cycles");
    println!();
    
    let args: Vec<String> = std::env::args().collect();
    let port: u16 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(9333);
    
    println!("Launching Chrome on port {}...", port);
    
    let mut browser = BrowserManager::new()
        .binary("chrome")
        .port(port)
        .launch()
        .await?;
    
    println!("Browser launched on port {}", browser.get_port());
    
    let runner = TestRunner::new()
        .max_concurrent(4)
        .build();
    
    println!("TestRunner ready with {} max concurrent", runner.get_max_concurrent());
    
    let ci = GitHubCI::new("token");
    println!("GitHubCI ready");
    
    println!();
    println!("Framework ready! Press Ctrl+C to exit.");
    
    // Keep running
    tokio::signal::ctrl_c().await?;
    
    browser.shutdown().await?;
    println!("Browser shut down cleanly");
    
    Ok(())
}
