// arxiv_request_endorse.rs - Follow endorsement request links for all categories
use cdp_rs::browser::BrowserManager;
use cdp_rs::page::Page;

const CHROME: &str = r"C:\Program Files\Google\Chrome\Application\chrome.exe";
const ARXIV_USER: &str = "YuriyGagarin";
const ARXIV_PASS: &str = "Klin_120478";

const CATEGORIES: &[&str] = &[
    "math.LO", "math.GM", "math.CO", "math.NT", "math.PR",
    "cs.AI", "cs.CR", "cs.LO", "cs.DS",
];

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("========================================");
    println!("  arXiv Request Endorsement (Rust)");
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
    println!("  Logged in");

    // For each category, go to submit page and click "request endorsement"
    for (i, cat) in CATEGORIES.iter().enumerate() {
        println!("\n[{}] Requesting endorsement for {}...", i + 1, cat);
        
        // Go to submit page
        page.navigate("https://arxiv.org/submit").await?;
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        
        // Fill form
        page.evaluate(r#"(() => {
            const userinfo = document.querySelector('input[name="userinfo"]');
            if (userinfo) { userinfo.checked = true; userinfo.dispatchEvent(new Event('change', {bubbles:true})); }
            const agree = document.querySelector('input[name="agree_terms_conditions"]');
            if (agree) { agree.checked = true; agree.dispatchEvent(new Event('change', {bubbles:true})); }
            const author = document.querySelector('input[name="is_author"][value="1"]');
            if (author) { author.checked = true; author.dispatchEvent(new Event('change', {bubbles:true})); }
            const license = document.querySelector('input[name="license"]');
            if (license) { license.checked = true; license.dispatchEvent(new Event('change', {bubbles:true})); }
        })()"#).await?;
        
        // Set archive
        let archive = if cat.starts_with("math") { "math" } else { "cs" };
        page.evaluate(&format!(
            r#"(() => {{
                const sel = document.querySelector('select[name="archive"]');
                if (sel) {{ sel.value = '{}'; sel.dispatchEvent(new Event('change', {{bubbles:true}})); }}
            }})()"#,
            archive
        )).await?;
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        
        // Set subject
        page.evaluate(&format!(
            r#"(() => {{
                const sel = document.querySelector('select[name="subject_class"]');
                if (sel) {{ sel.value = '{}'; sel.dispatchEvent(new Event('change', {{bubbles:true}})); }}
            }})()"#,
            cat
        )).await?;
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        
        // Click Continue to trigger endorsement error
        page.click("input[value=\"Continue\"]").await?;
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        
        // Find and click "request endorsement" link
        let endorse_url = page.evaluate(&format!(
            r#"(() => {{
                const links = document.querySelectorAll('a');
                for (const a of links) {{
                    if (a.textContent.includes('request endorsement') && a.href.includes('{}')) {{
                        return a.href;
                    }}
                }}
                // Try any request endorsement link
                for (const a of links) {{
                    if (a.textContent.includes('request endorsement')) {{
                        return a.href;
                    }}
                }}
                return null;
            }})()"#,
            cat
        )).await?;
        
        if let Some(url) = endorse_url.get("value").and_then(|v| v.as_str()) {
            println!("  Found: {}", &url[..url.len().min(100)]);
            page.navigate(url).await?;
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            
            // Take screenshot of endorsement request page
            page.screenshot(&format!("E:/1/arxiv_endorse_{}.png", cat.replace(".", "_"))).await?;
            
            // Check page content
            let content = page.evaluate("document.body.innerText.substring(0, 1000)").await?;
            println!("  Page: {:?}", content.get("value").and_then(|v| v.as_str()).unwrap_or("").chars().take(200).collect::<String>());
            
            // Look for email input and submit button
            let form_info = page.evaluate(r#"(() => {
                const inputs = document.querySelectorAll('input');
                const forms = document.querySelectorAll('form');
                return {
                    inputs: Array.from(inputs).map(i => i.name + ':' + i.type).join(', '),
                    forms: forms.length
                };
            })()"#).await?;
            println!("  Form: {:?}", form_info.get("value"));
        } else {
            println!("  No request link found");
        }
    }

    println!("\n========================================");
    println!("  Done! Check E:\\1\\arxiv_endorse_*.png");
    println!("========================================");

    Ok(())
}
