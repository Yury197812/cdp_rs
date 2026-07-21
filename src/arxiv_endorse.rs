// arxiv_endorse.rs - Request endorsement for multiple arXiv categories
use cdp_rs::browser::BrowserManager;
use cdp_rs::page::Page;

const CHROME: &str = r"C:\Program Files\Google\Chrome\Application\chrome.exe";
const ARXIV_USER: &str = "YuriyGagarin";
const ARXIV_PASS: &str = "Klin_120478";

// Categories to request endorsement for
const CATEGORIES: &[&str] = &[
    "math.LO",   // Logic
    "math.GM",   // General Mathematics
    "math.CO",   // Combinatorics
    "math.NT",   // Number Theory
    "math.PR",   // Probability
    "cs.AI",     // Artificial Intelligence
    "cs.CR",     // Cryptography and Security
    "cs.LO",     // Logic in Computer Science
    "cs.DS",     // Data Structures and Algorithms
];

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("========================================");
    println!("  arXiv Endorsement Requests (Rust)");
    println!("========================================\n");

    println!("[1] Launching Chrome...");
    let browser = BrowserManager::new().binary(CHROME).launch().await?;
    let conn = browser.connection().unwrap();
    let page = Page::new(conn.clone());

    // Login
    println!("[2] Logging in...");
    page.navigate("https://arxiv.org/user/login").await?;
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    page.fill("input[name=\"username\"]", ARXIV_USER).await?;
    page.fill("input[name=\"password\"]", ARXIV_PASS).await?;
    page.click("input[type=\"submit\"]").await?;
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
    println!("  Logged in: {}", page.get_url().await?);

    // Go to endorsement request page
    println!("[3] Opening endorsement page...");
    page.navigate("https://arxiv.org/auth/endorse.php").await?;
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    page.screenshot("E:/1/arxiv_endorse_01.png").await?;
    println!("  URL: {}", page.get_url().await?);

    // Check what's on the page
    let page_text = page.evaluate("document.body.innerText.substring(0, 1000)").await?;
    println!("  Page: {:?}", page_text.get("value").and_then(|v| v.as_str()).unwrap_or("").chars().take(300).collect::<String>());

    println!("\n========================================");
    println!("  Check E:\\1\\arxiv_endorse_01.png");
    println!("========================================");

    Ok(())
}
