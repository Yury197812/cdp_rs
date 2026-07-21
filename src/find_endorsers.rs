// find_endorsers.rs - Find potential endorsers on arXiv
use cdp_rs::browser::BrowserManager;
use cdp_rs::page::Page;

const CHROME: &str = r"C:\Program Files\Google\Chrome\Application\chrome.exe";

// Categories and their endorsement codes
const CATEGORIES: &[(&str, &str)] = &[
    ("math.LO", "NWTCV4"),
    ("math.GM", "CHECK_EMAIL"),
    ("math.CO", "CHECK_EMAIL"),
    ("math.NT", "CHECK_EMAIL"),
    ("math.PR", "CHECK_EMAIL"),
    ("cs.AI", "CHECK_EMAIL"),
    ("cs.CR", "CHECK_EMAIL"),
    ("cs.LO", "CHECK_EMAIL"),
    ("cs.DS", "CHECK_EMAIL"),
];

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("========================================");
    println!("  Finding arXiv Endorsers (Rust)");
    println!("========================================\n");

    println!("[1] Launching Chrome...");
    let browser = BrowserManager::new().binary(CHROME).launch().await?;
    let conn = browser.connection().unwrap();
    let page = Page::new(conn.clone());

    // For each category, find recent papers and their authors
    for (cat, code) in CATEGORIES {
        println!("\n=== {} (code: {}) ===", cat, code);
        
        // Search for recent papers in this category
        let search_url = format!("https://arxiv.org/search/?searchtype=author&query=&category={}&order=-announced_date_first", cat);
        page.navigate(&search_url).await?;
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
        
        // Get author names and paper titles
        let results = page.evaluate(r#"(() => {
            const papers = document.querySelectorAll('.arxiv-result');
            const data = [];
            for (let i = 0; i < Math.min(papers.length, 5); i++) {
                const p = papers[i];
                const title = p.querySelector('.title')?.textContent?.trim() || '';
                const authors = p.querySelector('.authors')?.textContent?.trim() || '';
                const link = p.querySelector('a[href^="/abs/"]')?.href || '';
                data.push({title: title.substring(0, 80), authors: authors.substring(0, 200), link});
            }
            return data;
        })()"#).await?;
        
        if let Some(val) = results.get("value") {
            if let Some(s) = val.as_str() {
                if let Ok(papers) = serde_json::from_str::<Vec<serde_json::Value>>(s) {
                    for (i, p) in papers.iter().enumerate() {
                        let title = p.get("title").and_then(|v| v.as_str()).unwrap_or("");
                        let authors = p.get("authors").and_then(|v| v.as_str()).unwrap_or("");
                        println!("  {}. {}", i + 1, title);
                        println!("     Authors: {}", &authors[..authors.len().min(150)]);
                    }
                }
            }
        }
    }

    println!("\n========================================");
    println!("  Done!");
    println!("========================================");

    Ok(())
}
