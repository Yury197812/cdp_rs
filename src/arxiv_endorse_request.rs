// arxiv_endorse_request.rs - Find and use endorsement request page
use cdp_rs::browser::BrowserManager;
use cdp_rs::page::Page;

const CHROME: &str = r"C:\Program Files\Google\Chrome\Application\chrome.exe";
const ARXIV_USER: &str = "YuriyGagarin";
const ARXIV_PASS: &str = "Klin_120478";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("========================================");
    println!("  arXiv Endorsement Request (Rust)");
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

    // Go to user page to see endorsement status
    println!("[3] Checking user page...");
    page.navigate("https://arxiv.org/user/YuriyGagarin").await?;
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    page.screenshot("E:/1/arxiv_user_page.png").await?;
    
    let user_info = page.evaluate("document.body.innerText.substring(0, 3000)").await?;
    println!("  User page:\n{}", user_info.get("value").and_then(|v| v.as_str()).unwrap_or(""));

    // Try to find endorsement request links
    println!("\n[4] Looking for endorsement links...");
    let links = page.evaluate(r#"(() => {
        const anchors = document.querySelectorAll('a');
        return Array.from(anchors)
            .filter(a => a.textContent.toLowerCase().includes('endorse') || a.href.includes('endorse'))
            .map(a => ({text: a.textContent.trim(), href: a.href}));
    })()"#).await?;
    println!("  Endorsement links: {:?}", links.get("value"));

    // Try the submit page to find the request endorsement link
    println!("\n[5] Going to submit page to find request link...");
    page.navigate("https://arxiv.org/submit").await?;
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
    
    // Fill form to trigger the endorsement error
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
    
    // Set archive to cs
    page.evaluate(r#"(() => {
        const sel = document.querySelector('select[name="archive"]');
        if (sel) { sel.value = 'cs'; sel.dispatchEvent(new Event('change', {bubbles:true})); }
    })()"#).await?;
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    
    // Set subject to cs.AI
    page.evaluate(r#"(() => {
        const sel = document.querySelector('select[name="subject_class"]');
        if (sel) { sel.value = 'cs.AI'; sel.dispatchEvent(new Event('change', {bubbles:true})); }
    })()"#).await?;
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    
    // Click Continue to trigger error
    page.click("input[value=\"Continue\"]").await?;
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
    
    // Now look for the "request endorsement" link
    let request_links = page.evaluate(r#"(() => {
        const anchors = document.querySelectorAll('a');
        return Array.from(anchors)
            .filter(a => a.textContent.toLowerCase().includes('request') || a.textContent.toLowerCase().includes('endorse'))
            .map(a => ({text: a.textContent.trim().substring(0, 100), href: a.href}));
    })()"#).await?;
    println!("  Request links: {:?}", request_links.get("value"));
    
    page.screenshot("E:/1/arxiv_endorse_request.png").await?;

    println!("\n========================================");
    println!("  Check screenshots in E:\\1\\arxiv_*.png");
    println!("========================================");

    Ok(())
}
