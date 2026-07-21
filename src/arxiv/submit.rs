// arxiv_submit.rs - Automated arXiv submission via CDP
use cdp_rs::browser::BrowserManager;
use cdp_rs::page::Page;
use cdp_rs::upload::UploadHandler;

const CHROME: &str = r"C:\Program Files\Google\Chrome\Application\chrome.exe";
const ARXIV_USER: &str = "YuriyGagarin";
const ARXIV_PASS: &str = "Klin_120478";
const SUBMISSION_FILE: &str = r"E:\1\arxiv_final\proofs_1001.tar.gz";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("========================================");
    println!("  arXiv Auto-Submit (Rust + CDP)");
    println!("========================================\n");

    // Launch browser
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

    // Start submission
    println!("[3] Opening submit page...");
    page.navigate("https://arxiv.org/submit").await?;
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // Fill form via DOM
    println!("[4] Filling form via DOM...");
    
    // Check all boxes and select radios via JS
    let fill_result = page.evaluate(r#"(() => {
        const results = [];
        
        // 1. Verify checkbox
        const userinfo = document.querySelector('input[name="userinfo"]');
        if (userinfo) { userinfo.checked = true; userinfo.dispatchEvent(new Event('change', {bubbles:true})); results.push('userinfo:true'); }
        
        // 2. Agree checkbox
        const agree = document.querySelector('input[name="agree_terms_conditions"]');
        if (agree) { agree.checked = true; agree.dispatchEvent(new Event('change', {bubbles:true})); results.push('agree:true'); }
        
        // 3. Authorship radio (value=1 = "I am author")
        const author = document.querySelector('input[name="is_author"][value="1"]');
        if (author) { author.checked = true; author.dispatchEvent(new Event('change', {bubbles:true})); results.push('author:1'); }
        
        // 4. License radio (first one = CC BY)
        const license = document.querySelector('input[name="license"]');
        if (license) { license.checked = true; license.dispatchEvent(new Event('change', {bubbles:true})); results.push('license:true'); }
        
        // 5. Archive dropdown - use cs (Computer Science) for AI
        const archiveSel = document.querySelector('select[name="archive"]');
        if (archiveSel) {
            const setter = Object.getOwnPropertyDescriptor(window.HTMLSelectElement.prototype, 'value').set;
            setter.call(archiveSel, 'cs');
            archiveSel.dispatchEvent(new Event('change', {bubbles:true}));
            results.push('archive:cs');
        }
        
        return results;
    })()"#).await?;
    println!("  Filled: {:?}", fill_result.get("value"));
    
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Subject class dropdown - wait for cs options to load
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Select cs.AI
    let subject_result = page.evaluate(r#"(() => {
        const selects = document.querySelectorAll('select');
        let sel = null;
        for (const s of selects) {
            if (s.name === 'subject_class') { sel = s; break; }
        }
        if (!sel && selects.length > 1) { sel = selects[1]; }
        if (!sel) return 'no subject select found';
        
        const opts = Array.from(sel.options);
        const values = opts.map(o => o.value + ':' + o.text);
        
        // Find cs.AI
        const ai = opts.find(o => o.value === 'cs.AI');
        const target = ai || opts.find(o => o.value && !o.value.startsWith('-'));
        
        if (target && target.value) {
            const setter = Object.getOwnPropertyDescriptor(window.HTMLSelectElement.prototype, 'value').set;
            setter.call(sel, target.value);
            sel.dispatchEvent(new Event('change', {bubbles:true}));
            return 'selected:' + target.value + ' all:' + values.join(',');
        }
        return 'no matching option in: ' + values.join(',');
    })()"#).await?;
    println!("  Subject: {:?}", subject_result.get("value"));
    
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Verify state
    let state = page.evaluate(r#"JSON.stringify({
        userinfo: document.querySelector('input[name="userinfo"]')?.checked,
        agree: document.querySelector('input[name="agree_terms_conditions"]')?.checked,
        author: document.querySelector('input[name="is_author"]:checked')?.value,
        license: document.querySelector('input[name="license"]:checked')?.value,
        archive: document.querySelector('select[name="archive"]')?.value,
        subject: document.querySelector('select[name="subject_class"]')?.value
    })()"#).await?;
    println!("  State: {}", state.get("value").and_then(|v| v.as_str()).unwrap_or("?"));
    
    page.screenshot("E:/1/arxiv_rust_03_filled.png").await?;

    // Click Continue
    println!("[5] Clicking Continue...");
    page.click("input[value=\"Continue\"]").await?;
    tokio::time::sleep(tokio::time::Duration::from_secs(4)).await;
    
    let url = page.get_url().await?;
    println!("  URL: {}", url);
    page.screenshot("E:/1/arxiv_rust_04_after_continue.png").await?;

    // Check for errors
    let errors = page.evaluate(r#"(() => {
        const errs = document.querySelectorAll('.error, .alert-danger, [class*="error"]');
        return Array.from(errs).map(e => e.textContent.trim()).join(' | ');
    })()"#).await?;
    let errors_str = errors.get("value").and_then(|v| v.as_str()).unwrap_or("");
    if !errors_str.is_empty() {
        println!("\n  Errors: {}", errors_str);
    }

    // If on add_files page, upload
    if url.contains("add_files") || url.contains("files") {
        println!("[6] Uploading file...");
        let upload = UploadHandler::new(conn.clone());
        upload.set_files_via_dom("input[type=\"file\"]", &[SUBMISSION_FILE]).await?;
        println!("  Uploaded: {}", SUBMISSION_FILE);
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
        page.screenshot("E:/1/arxiv_rust_05_uploaded.png").await?;
        
        page.click("input[value=\"Continue\"]").await?;
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
        page.screenshot("E:/1/arxiv_rust_06_files_done.png").await?;
        println!("  URL: {}", page.get_url().await?);
    } else {
        println!("\n[!] Still on start page");
        // Scroll to top to see errors
        page.evaluate("window.scrollTo(0, 0)").await?;
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        page.screenshot("E:/1/arxiv_rust_04_errors_top.png").await?;
    }

    println!("\n========================================");
    println!("  Done! Check E:\\1\\arxiv_rust_*.png");
    println!("========================================");

    Ok(())
}
