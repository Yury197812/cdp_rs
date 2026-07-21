use cdp_rs::browser::{BrowserManager, CookieEntry};
use cdp_rs::network_intercept::NetworkInterceptor;
use cdp_rs::network;
use cdp_rs::browser_pool::BrowserPool;
use cdp_rs::page::Page;
use cdp_rs::gmail_oauth::GmailOAuth;
use cdp_rs::pdf::{PdfGenerator, PdfOptions};
use cdp_rs::cookie::{CookieManager, CookieJar, Cookie};
use cdp_rs::screenshot::{ScreenshotCapture, ScreenshotOptions};
use std::collections::HashMap;
use std::sync::Arc;

const CHROME: &str = r"C:\Program Files\Google\Chrome\Application\chrome.exe";

/// Parse --proxy <url> from args
fn find_proxy(args: &[String]) -> Option<String> {
    args.iter()
        .position(|a| a == "--proxy")
        .and_then(|i| args.get(i + 1))
        .cloned()
}

/// Parse --header "Key: Value" from args (repeatable)
fn find_headers(args: &[String]) -> HashMap<String, String> {
    let mut headers = HashMap::new();
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--header" {
            if let Some(val) = args.get(i + 1) {
                if let Some((key, value)) = val.split_once(':') {
                    headers.insert(key.trim().to_string(), value.trim().to_string());
                }
                i += 2;
                continue;
            }
        }
        i += 1;
    }
    headers
}

/// Parse --cookie "name=value" from args (repeatable)
fn find_cookies(args: &[String]) -> Vec<CookieEntry> {
    let mut cookies = Vec::new();
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--cookie" {
            if let Some(val) = args.get(i + 1) {
                let (name, value) = val.split_once('=').unwrap_or((val, ""));
                cookies.push(CookieEntry {
                    name: name.to_string(),
                    value: value.to_string(),
                    domain: None,
                    path: None,
                });
                i += 2;
                continue;
            }
        }
        i += 1;
    }
    cookies
}

/// Parse --quality <n> from args
fn find_quality(args: &[String]) -> Option<u32> {
    args.iter()
        .position(|a| a == "--quality")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse().ok())
}

/// Parse --scale <n> from args
fn find_scale(args: &[String]) -> Option<f64> {
    args.iter()
        .position(|a| a == "--scale")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse().ok())
}

/// Parse --element <selector> from args
fn find_element(args: &[String]) -> Option<String> {
    args.iter()
        .position(|a| a == "--element")
        .and_then(|i| args.get(i + 1))
        .cloned()
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let command = args.get(1).map(|s| s.as_str()).unwrap_or("help");
    let proxy = find_proxy(&args);
    let headers = find_headers(&args);
    let cookies = find_cookies(&args);

    match command {
        "gmail-oauth" => {
            let sub = args.get(2).map(|s| s.as_str()).unwrap_or("setup");
            match sub {
                "auth" => {
                    println!("Running Gmail OAuth authentication...");
                    let mut bm = BrowserManager::new().binary(CHROME);
                    if let Some(ref p) = proxy { bm = bm.proxy(p); }
                    if !headers.is_empty() { bm = bm.headers(headers); }
                    if !cookies.is_empty() { bm = bm.cookies(cookies); }
                    let browser = bm.launch().await?;
                    let conn = browser.connection().unwrap();
                    let oauth = GmailOAuth::new(conn);
                    oauth.run().await?;
                }
                _ => {
                    println!("Usage: cdp_rs gmail-oauth auth [--proxy <url>] [--header \"K: V\"] [--cookie \"n=v\"]");
                }
            }
        }
        "intercept" => {
            println!("Starting browser with ad-blocker...");
            let mut bm = BrowserManager::new().binary(CHROME);
            if let Some(ref p) = proxy { bm = bm.proxy(p); }
            if !headers.is_empty() { bm = bm.headers(headers); }
            if !cookies.is_empty() { bm = bm.cookies(cookies); }
            let browser = bm.launch().await?;
            let conn = browser.connection().unwrap();

            let interceptor = NetworkInterceptor::new(conn.clone());
            interceptor.enable().await?;
            interceptor.block_ads().await;
            println!("Ad-blocker enabled! Browsing...");

            let mut rx = browser.subscribe_events().unwrap();
            while let Ok(event) = rx.recv().await {
                interceptor.handle_event(&event).await?;
            }
        }
        "pool" => {
            let size: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(2);
            println!("Starting browser pool (size={})...", size);
            let mut pool = BrowserPool::new(size, 9500, true).with_chrome_path(CHROME);
            if let Some(ref p) = proxy { pool = pool.with_proxy(p); }
            if !headers.is_empty() { pool = pool.with_headers(headers); }
            if !cookies.is_empty() {
                for c in &cookies { pool = pool.with_cookie(&c.name, &c.value); }
            }
            let browser = pool.acquire().await?;
            println!("Acquired browser on port {}", browser.port);

            let stats = pool.stats().await;
            println!("Pool: {}", stats);

            let page = Page::new(browser.connection.clone());
            page.navigate("https://example.com").await?;
            println!("Title: {}", page.get_title().await?);
        }
        "page" => {
            let url = args.get(2).map(|s| s.as_str()).unwrap_or("https://example.com");
            println!("Opening: {}", url);
            let mut bm = BrowserManager::new().binary(CHROME);
            if let Some(ref p) = proxy { bm = bm.proxy(p); }
            if !headers.is_empty() { bm = bm.headers(headers); }
            if !cookies.is_empty() { bm = bm.cookies(cookies); }
            let browser = bm.launch().await?;
            let conn = browser.connection().unwrap();
            let page = Page::new(conn.clone());
            page.navigate(url).await?;
            println!("Title: {}", page.get_title().await?);

            if let Some(path) = args.get(3) {
                let cap = ScreenshotCapture::new(conn);
                let mut opts = ScreenshotOptions::png();
                if args.contains(&"--full-page".to_string()) { opts = opts.full_page(); }
                if args.contains(&"--transparent".to_string()) { opts = opts.omit_background(); }
                if let Some(q) = find_quality(&args) {
                    opts = ScreenshotOptions::jpeg(q);
                }
                cap.save(path, &opts).await?;
                println!("Screenshot saved: {}", path);
            }
        }
        "screenshot" => {
            let url = args.get(2).map(|s| s.as_str()).unwrap_or("https://example.com");
            let output = args.get(3).map(|s| s.as_str()).unwrap_or("screenshot.png");
            println!("Screenshot from: {}", url);

            let mut bm = BrowserManager::new().binary(CHROME);
            if let Some(ref p) = proxy { bm = bm.proxy(p); }
            if !headers.is_empty() { bm = bm.headers(headers); }
            if !cookies.is_empty() { bm = bm.cookies(cookies); }
            let browser = bm.launch().await?;
            let conn = browser.connection().unwrap();
            let page = Page::new(conn.clone());
            page.navigate(url).await?;

            let cap = ScreenshotCapture::new(conn);
            let mut opts = ScreenshotOptions::png();

            if args.contains(&"--full-page".to_string()) { opts = opts.full_page(); }
            if args.contains(&"--transparent".to_string()) { opts = opts.omit_background(); }
            if args.contains(&"--jpeg".to_string()) {
                let q = find_quality(&args).unwrap_or(80);
                opts = ScreenshotOptions::jpeg(q);
            }

            // Element screenshot: --element "#selector"
            if let Some(selector) = find_element(&args) {
                cap.element_screenshot(&selector, output).await?;
                println!("Element screenshot saved: {}", output);
            } else {
                cap.save(output, &opts).await?;
                println!("Screenshot saved: {}", output);
            }
        }
        "pdf" => {
            let url = args.get(2).map(|s| s.as_str()).unwrap_or("https://example.com");
            let output = args.get(3).map(|s| s.as_str()).unwrap_or("output.pdf");
            println!("Generating PDF from: {}", url);

            let mut bm = BrowserManager::new().binary(CHROME);
            if let Some(ref p) = proxy { bm = bm.proxy(p); }
            if !headers.is_empty() { bm = bm.headers(headers); }
            if !cookies.is_empty() { bm = bm.cookies(cookies); }
            let browser = bm.launch().await?;
            let conn = browser.connection().unwrap();
            let page = Page::new(conn.clone());
            page.navigate(url).await?;

            let pdf = PdfGenerator::new(conn);
            let mut opts = PdfOptions::default();
            if args.contains(&"--landscape".to_string()) { opts = opts.landscape(); }
            if let Some(scale) = find_scale(&args) { opts = opts.scale(scale); }

            pdf.save(output, &opts).await?;
            println!("PDF saved: {}", output);
        }
        "cookies" => {
            let sub = args.get(2).map(|s| s.as_str()).unwrap_or("list");
            match sub {
                "list" => {
                    let url = args.get(3).map(|s| s.as_str()).unwrap_or("https://example.com");
                    println!("Cookies for: {}", url);

                    let mut bm = BrowserManager::new().binary(CHROME);
                    if let Some(ref p) = proxy { bm = bm.proxy(p); }
                    if !headers.is_empty() { bm = bm.headers(headers); }
                    if !cookies.is_empty() { bm = bm.cookies(cookies); }
                    let browser = bm.launch().await?;
                    let conn = browser.connection().unwrap();

                    let cookie_mgr = CookieManager::new(conn);
                    let result = cookie_mgr.get_cookies(&[url]).await?;
                    println!("Found {} cookies:", result.len());
                    for c in &result {
                        println!("  {} = {} (domain: {})", c.name, c.value, c.domain);
                    }
                }
                "export" => {
                    let url = args.get(3).map(|s| s.as_str()).unwrap_or("https://example.com");
                    let output = args.get(4).map(|s| s.as_str()).unwrap_or("cookies.json");
                    println!("Exporting cookies from: {}", url);

                    let mut bm = BrowserManager::new().binary(CHROME);
                    if let Some(ref p) = proxy { bm = bm.proxy(p); }
                    if !headers.is_empty() { bm = bm.headers(headers); }
                    if !cookies.is_empty() { bm = bm.cookies(cookies); }
                    let browser = bm.launch().await?;
                    let conn = browser.connection().unwrap();

                    let cookie_mgr = CookieManager::new(conn);
                    if output.ends_with(".txt") {
                        cookie_mgr.export_netscape(&[url], output).await?;
                    } else {
                        cookie_mgr.export_json(&[url], output).await?;
                    }
                    println!("Exported to: {}", output);
                }
                "import" => {
                    let input = args.get(3).map(|s| s.as_str()).unwrap_or("cookies.json");
                    let url = args.get(4).map(|s| s.as_str()).unwrap_or("http://localhost");
                    println!("Importing cookies from: {}", input);

                    let mut bm = BrowserManager::new().binary(CHROME);
                    if let Some(ref p) = proxy { bm = bm.proxy(p); }
                    let browser = bm.launch().await?;
                    let conn = browser.connection().unwrap();

                    // Navigate to domain first
                    let page = Page::new(conn.clone());
                    page.navigate(url).await?;

                    let cookie_mgr = CookieManager::new(conn);
                    let count = if input.ends_with(".txt") {
                        cookie_mgr.import_netscape(input).await?
                    } else {
                        cookie_mgr.import_json(input).await?
                    };
                    println!("Imported {} cookies", count);
                }
                "set" => {
                    let name = args.get(3).cloned().unwrap_or_default();
                    let value = args.get(4).cloned().unwrap_or_default();
                    let url = args.get(5).map(|s| s.as_str()).unwrap_or("http://localhost");
                    if name.is_empty() {
                        println!("Usage: cdp_rs cookies set <name> <value> [url]");
                        return Ok(());
                    }
                    println!("Setting cookie: {}={}", name, value);

                    let mut bm = BrowserManager::new().binary(CHROME);
                    if let Some(ref p) = proxy { bm = bm.proxy(p); }
                    let browser = bm.launch().await?;
                    let conn = browser.connection().unwrap();

                    let page = Page::new(conn.clone());
                    page.navigate(url).await?;

                    let cookie_mgr = CookieManager::new(conn);
                    cookie_mgr.set_cookie(&Cookie::new(&name, &value)).await?;
                    println!("Cookie set!");
                }
                "delete" => {
                    let name = args.get(3).cloned().unwrap_or_default();
                    let url = args.get(4).map(|s| s.as_str()).unwrap_or("http://localhost");
                    if name.is_empty() {
                        println!("Usage: cdp_rs cookies delete <name> [url]");
                        return Ok(());
                    }
                    println!("Deleting cookie: {}", name);

                    let mut bm = BrowserManager::new().binary(CHROME);
                    if let Some(ref p) = proxy { bm = bm.proxy(p); }
                    let browser = bm.launch().await?;
                    let conn = browser.connection().unwrap();

                    let page = Page::new(conn.clone());
                    page.navigate(url).await?;

                    let cookie_mgr = CookieManager::new(conn);
                    cookie_mgr.delete_by_name(&name).await?;
                    println!("Cookie deleted!");
                }
                "clear" => {
                    println!("Clearing all cookies...");

                    let mut bm = BrowserManager::new().binary(CHROME);
                    if let Some(ref p) = proxy { bm = bm.proxy(p); }
                    let browser = bm.launch().await?;
                    let conn = browser.connection().unwrap();

                    let cookie_mgr = CookieManager::new(conn);
                    cookie_mgr.clear_all().await?;
                    println!("All cookies cleared!");
                }
                _ => {
                    // Legacy: treat as URL
                    let url = sub;
                    println!("Cookies for: {}", url);

                    let mut bm = BrowserManager::new().binary(CHROME);
                    if let Some(ref p) = proxy { bm = bm.proxy(p); }
                    if !headers.is_empty() { bm = bm.headers(headers); }
                    if !cookies.is_empty() { bm = bm.cookies(cookies); }
                    let browser = bm.launch().await?;
                    let conn = browser.connection().unwrap();

                    let cookie_mgr = CookieManager::new(conn);
                    let result = cookie_mgr.get_cookies(&[url]).await?;
                    println!("Found {} cookies:", result.len());
                    for c in &result {
                        println!("  {} = {} (domain: {})", c.name, c.value, c.domain);
                    }
                }
            }
        }
        "cookie-jar" => {
            let sub = args.get(2).map(|s| s.as_str()).unwrap_or("help");
            match sub {
                "create" => {
                    let mut jar = CookieJar::new();
                    // Parse --cookie flags
                    for c in &cookies {
                        jar.add(Cookie::new(&c.name, &c.value));
                    }
                    let output = args.get(3).map(|s| s.as_str()).unwrap_or("cookies.json");
                    jar.save_json(output).await?;
                    println!("Created cookie jar with {} cookies: {}", jar.len(), output);
                }
                "view" => {
                    let input = args.get(3).map(|s| s.as_str()).unwrap_or("cookies.json");
                    let jar = CookieJar::load_json(input).await?;
                    println!("Cookie jar ({} cookies):", jar.len());
                    for c in jar.all() {
                        println!("  {} = {} (domain: {})", c.name, c.value, c.domain);
                    }
                    println!("\nCookie header: {}", jar.to_cookie_header());
                }
                "header" => {
                    let input = args.get(3).map(|s| s.as_str()).unwrap_or("cookies.json");
                    let jar = CookieJar::load_json(input).await?;
                    println!("{}", jar.to_cookie_header());
                }
                "merge" => {
                    let input1 = args.get(3).map(|s| s.as_str()).unwrap_or("a.json");
                    let input2 = args.get(4).map(|s| s.as_str()).unwrap_or("b.json");
                    let output = args.get(5).map(|s| s.as_str()).unwrap_or("merged.json");
                    let mut jar1 = CookieJar::load_json(input1).await?;
                    let jar2 = CookieJar::load_json(input2).await?;
                    jar1.merge(&jar2);
                    jar1.save_json(output).await?;
                    println!("Merged into {} ({} cookies)", output, jar1.len());
                }
                _ => {
                    println!("Usage: cdp_rs cookie-jar <create|view|header|merge> [args]");
                }
            }
        }
        "network" => {
            let sub = args.get(2).map(|s| s.as_str()).unwrap_or("log");
            match sub {
                "log" => {
                    let url = args.get(3).map(|s| s.as_str()).unwrap_or("https://example.com");
                    let output = args.get(4).map(|s| s.as_str()).unwrap_or("network.har");
                    println!("Recording network for: {} (Ctrl+C to stop)", url);

                    let mut bm = BrowserManager::new().binary(CHROME);
                    if let Some(ref p) = proxy { bm = bm.proxy(p); }
                    if !headers.is_empty() { bm = bm.headers(headers); }
                    if !cookies.is_empty() { bm = bm.cookies(cookies); }
                    let browser = bm.launch().await?;
                    let conn = browser.connection().unwrap();

                    let interceptor = Arc::new(network::NetworkInterceptor::with_options(conn.clone(), network::InterceptOptions::default()));
                    interceptor.enable().await?;

                    let page = Page::new(conn.clone());
                    page.navigate(url).await?;

                    let mut rx = browser.subscribe_events().unwrap();
                    println!("Recording... Press Ctrl+C to stop and save.");

                    let interceptor2 = interceptor.clone();
                    tokio::spawn(async move {
                        while let Ok(event) = rx.recv().await {
                            let _ = interceptor2.handle_event(&event).await;
                        }
                    });

                    tokio::signal::ctrl_c().await?;

                    let count = interceptor.request_count().await;
                    let bytes = interceptor.total_bytes().await;
                    println!("\nRecorded {} requests ({} bytes)", count, bytes);

                    interceptor.save_har(output).await?;
                    println!("HAR saved to: {}", output);
                }
                "monitor" => {
                    let url = args.get(3).map(|s| s.as_str()).unwrap_or("https://example.com");
                    println!("Monitoring network for: {} (Ctrl+C to stop)", url);

                    let mut bm = BrowserManager::new().binary(CHROME);
                    if let Some(ref p) = proxy { bm = bm.proxy(p); }
                    if !headers.is_empty() { bm = bm.headers(headers); }
                    if !cookies.is_empty() { bm = bm.cookies(cookies); }
                    let browser = bm.launch().await?;
                    let conn = browser.connection().unwrap();

                    let interceptor = network::NetworkInterceptor::new(conn.clone());
                    interceptor.enable().await?;

                    let page = Page::new(conn.clone());
                    page.navigate(url).await?;

                    let mut rx = browser.subscribe_events().unwrap();
                    println!("{:<8} {:<6} {:<50} {:<10}", "METHOD", "STATUS", "URL", "SIZE");
                    println!("{}", "-".repeat(80));

                    tokio::spawn(async move {
                        while let Ok(event) = rx.recv().await {
                            if event.method == "Network.responseReceived" {
                                let url = event.params["response"]["url"].as_str().unwrap_or("");
                                let status = event.params["response"]["status"].as_u64().unwrap_or(0);
                                let method = event.params["type"].as_str().unwrap_or("");
                                println!("{:<8} {:<6} {:<50} {:<10}", method, status, &url[..50.min(url.len())], "-");
                            }
                        }
                    });

                    tokio::signal::ctrl_c().await?;
                    println!("\nDone.");
                }
                "block" => {
                    let pattern = args.get(3).map(|s| s.as_str()).unwrap_or("*ads*");
                    let url = args.get(4).map(|s| s.as_str()).unwrap_or("https://example.com");
                    println!("Blocking pattern: {} on {}", pattern, url);

                    let mut bm = BrowserManager::new().binary(CHROME);
                    if let Some(ref p) = proxy { bm = bm.proxy(p); }
                    let browser = bm.launch().await?;
                    let conn = browser.connection().unwrap();

                    let interceptor = network::NetworkInterceptor::new(conn.clone());
                    interceptor.enable().await?;
                    interceptor.enable_fetch(&["*"]).await?;
                    interceptor.block(pattern).await;

                    let page = Page::new(conn.clone());
                    page.navigate(url).await?;

                    let mut rx = browser.subscribe_events().unwrap();
                    tokio::spawn(async move {
                        while let Ok(event) = rx.recv().await {
                            let _ = interceptor.handle_event(&event).await;
                        }
                    });

                    tokio::signal::ctrl_c().await?;
                }
                "mock" => {
                    let pattern = args.get(3).map(|s| s.as_str()).unwrap_or("*api*");
                    let body = args.get(4).map(|s| s.as_str()).unwrap_or("{\"mocked\":true}");
                    let url = args.get(5).map(|s| s.as_str()).unwrap_or("https://example.com");
                    println!("Mocking pattern: {} with {}", pattern, body);

                    let mut bm = BrowserManager::new().binary(CHROME);
                    if let Some(ref p) = proxy { bm = bm.proxy(p); }
                    let browser = bm.launch().await?;
                    let conn = browser.connection().unwrap();

                    let interceptor = network::NetworkInterceptor::new(conn.clone());
                    interceptor.enable().await?;
                    interceptor.enable_fetch(&["*"]).await?;
                    interceptor.mock(pattern, 200, body, "application/json").await;

                    let page = Page::new(conn.clone());
                    page.navigate(url).await?;

                    let mut rx = browser.subscribe_events().unwrap();
                    tokio::spawn(async move {
                        while let Ok(event) = rx.recv().await {
                            let _ = interceptor.handle_event(&event).await;
                        }
                    });

                    tokio::signal::ctrl_c().await?;
                }
                "stats" => {
                    let url = args.get(3).map(|s| s.as_str()).unwrap_or("https://example.com");
                    println!("Collecting stats for: {}", url);

                    let mut bm = BrowserManager::new().binary(CHROME);
                    if let Some(ref p) = proxy { bm = bm.proxy(p); }
                    let browser = bm.launch().await?;
                    let conn = browser.connection().unwrap();

                    let interceptor = network::NetworkInterceptor::new(conn.clone());
                    interceptor.enable().await?;

                    let page = Page::new(conn.clone());
                    page.navigate(url).await?;

                    // Wait for page to load
                    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

                    let total = interceptor.request_count().await;
                    let bytes = interceptor.total_bytes().await;
                    let failed = interceptor.failed_entries().await;
                    let xhr = interceptor.entries_by_method("XHR").await;
                    let img = interceptor.entries_for("image/").await;

                    println!("\n=== Network Stats ===");
                    println!("Total requests: {}", total);
                    println!("Total bytes: {}", bytes);
                    println!("Failed requests: {}", failed.len());
                    println!("XHR/Fetch requests: {}", xhr.len());
                    println!("Image requests: {}", img.len());
                }
                _ => {
                    println!("Usage: cdp_rs network <log|monitor|block|mock|stats> [url]");
                }
            }
        }
        "js" => {
            let sub = args.get(2).map(|s| s.as_str()).unwrap_or("eval");
            match sub {
                "eval" => {
                    let expression = args.get(3).map(|s| s.as_str()).unwrap_or("1 + 1");
                    let url = args.get(4).map(|s| s.as_str()).unwrap_or("about:blank");
                    println!("Evaluating: {}", expression);

                    let mut bm = BrowserManager::new().binary(CHROME);
                    if let Some(ref p) = proxy { bm = bm.proxy(p); }
                    if !headers.is_empty() { bm = bm.headers(headers); }
                    if !cookies.is_empty() { bm = bm.cookies(cookies); }
                    let browser = bm.launch().await?;
                    let conn = browser.connection().unwrap();

                    let js = cdp_rs::js::JsEngine::new(conn.clone());
                    js.enable().await?;

                    let page = Page::new(conn);
                    page.navigate(url).await?;

                    match js.eval(expression).await {
                        Ok(result) => println!("Result: {}", serde_json::to_string_pretty(&result).unwrap_or_default()),
                        Err(e) => println!("Error: {}", e),
                    }
                }
                "console" => {
                    let url = args.get(3).map(|s| s.as_str()).unwrap_or("https://example.com");
                    println!("Monitoring console for: {} (Ctrl+C to stop)", url);

                    let mut bm = BrowserManager::new().binary(CHROME);
                    if let Some(ref p) = proxy { bm = bm.proxy(p); }
                    if !headers.is_empty() { bm = bm.headers(headers); }
                    if !cookies.is_empty() { bm = bm.cookies(cookies); }
                    let browser = bm.launch().await?;
                    let conn = browser.connection().unwrap();

                    let js = Arc::new(cdp_rs::js::JsEngine::new(conn.clone()));
                    js.enable().await?;

                    let page = Page::new(conn.clone());
                    page.navigate(url).await?;

                    let mut rx_console = js.on_console();
                    tokio::spawn(async move {
                        while let Ok(entry) = rx_console.recv().await {
                            println!("[{}] {}", entry.level.to_uppercase(), entry.text);
                        }
                    });

                    let mut rx = browser.subscribe_events().unwrap();
                    let js2 = js.clone();
                    tokio::spawn(async move {
                        while let Ok(event) = rx.recv().await {
                            let _ = js2.handle_event(&event).await;
                        }
                    });

                    tokio::signal::ctrl_c().await?;

                    let logs = js.console_logs().await;
                    println!("\n=== Console Summary ===");
                    println!("Total messages: {}", logs.len());
                    println!("Errors: {}", js.console_errors().await.len());
                    println!("Warnings: {}", js.console_warnings().await.len());
                }
                "run" => {
                    let snippet_name = args.get(3).map(|s| s.as_str()).unwrap_or("title");
                    let url = args.get(4).map(|s| s.as_str()).unwrap_or("https://example.com");
                    println!("Running snippet: {} on {}", snippet_name, url);

                    let mut bm = BrowserManager::new().binary(CHROME);
                    if let Some(ref p) = proxy { bm = bm.proxy(p); }
                    if !headers.is_empty() { bm = bm.headers(headers); }
                    if !cookies.is_empty() { bm = bm.cookies(cookies); }
                    let browser = bm.launch().await?;
                    let conn = browser.connection().unwrap();

                    let js = cdp_rs::js::JsEngine::new(conn.clone());
                    js.enable().await?;

                    let page = Page::new(conn);
                    page.navigate(url).await?;

                    match js.run_snippet(snippet_name).await {
                        Ok(result) => println!("Result: {}", serde_json::to_string_pretty(&result).unwrap_or_default()),
                        Err(e) => println!("Error: {}", e),
                    }
                }
                "inject" => {
                    let source = args.get(3).map(|s| s.as_str()).unwrap_or("console.log('injected')");
                    let url = args.get(4).map(|s| s.as_str()).unwrap_or("https://example.com");
                    println!("Injecting script on: {}", url);

                    let mut bm = BrowserManager::new().binary(CHROME);
                    if let Some(ref p) = proxy { bm = bm.proxy(p); }
                    let browser = bm.launch().await?;
                    let conn = browser.connection().unwrap();

                    let js = cdp_rs::js::JsEngine::new(conn.clone());
                    js.enable().await?;
                    js.add_script_to_evaluate_on_new_document(source).await?;

                    let page = Page::new(conn);
                    page.navigate(url).await?;
                    println!("Script injected! It will run on every page load.");
                }
                "wait" => {
                    let condition = args.get(3).map(|s| s.as_str()).unwrap_or("document.readyState === 'complete'");
                    let timeout: u64 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(30000);
                    let url = args.get(5).map(|s| s.as_str()).unwrap_or("https://example.com");
                    println!("Waiting for: {} ({}ms timeout)", condition, timeout);

                    let mut bm = BrowserManager::new().binary(CHROME);
                    if let Some(ref p) = proxy { bm = bm.proxy(p); }
                    let browser = bm.launch().await?;
                    let conn = browser.connection().unwrap();

                    let js = cdp_rs::js::JsEngine::new(conn.clone());
                    js.enable().await?;

                    let page = Page::new(conn);
                    page.navigate(url).await?;

                    match js.wait_for(condition, timeout).await {
                        Ok(()) => println!("Condition met!"),
                        Err(e) => println!("Error: {}", e),
                    }
                }
                "snippets" => {
                    println!("Available snippets:");
                    for name in &["cookies", "localstorage", "sessionstorage", "viewport", "scroll", "title", "url", "all_links", "all_images", "all_forms", "performance", "errors"] {
                        let snippet = cdp_rs::js::JsEngine::snippet(name);
                        println!("  {:<20} {}", name, snippet);
                    }
                }
                _ => {
                    println!("Usage: cdp_rs js <eval|console|run|inject|wait|snippets> [args]");
                }
            }
        }
        "help" | "--help" => {
            println!("CDP-RS: Browser Automation Framework (Rust)");
            println!();
            println!("Usage: cdp_rs <command> [options]");
            println!();
            println!("Commands:");
            println!("  gmail-oauth auth              — Gmail OAuth auto-setup");
            println!("  intercept                     — Browse with ad-blocker");
            println!("  pool [size]                   — Browser pool demo");
            println!("  page <url> [screenshot]       — Open URL, optional screenshot");
            println!("  screenshot <url> <output>     — Capture screenshot");
            println!("  pdf <url> [output]            — Generate PDF from URL");
            println!("  js eval <expr> [url]          — Evaluate JavaScript");
            println!("  js console <url>              — Monitor console output");
            println!("  js run <snippet> [url]        — Run built-in snippet");
            println!("  js inject <source> [url]      — Inject script on load");
            println!("  js wait <condition> [timeout] [url] — Wait for JS condition");
            println!("  js snippets                   — List available snippets");
            println!("  network log <url> [har]       — Record network to HAR file");
            println!("  network monitor <url>         — Live network monitor");
            println!("  network block <pattern> [url] — Block matching requests");
            println!("  network mock <pattern> <body> [url] — Mock responses");
            println!("  network stats <url>           — Collect network statistics");
            println!("  cookies list <url>            — List cookies for URL");
            println!("  cookies export <url> <file>   — Export cookies (json/netscape)");
            println!("  cookies import <file> [url]   — Import cookies from file");
            println!("  cookies set <n> <v> [url]     — Set a cookie");
            println!("  cookies delete <name> [url]   — Delete cookie by name");
            println!("  cookies clear                 — Clear all cookies");
            println!("  cookie-jar create <file>      — Create jar from --cookie flags");
            println!("  cookie-jar view <file>        — View jar contents");
            println!("  cookie-jar header <file>      — Print Cookie header");
            println!("  cookie-jar merge <a> <b> <out> — Merge two jars");
            println!("  help                          — Show this help");
            println!();
            println!("Screenshot options:");
            println!("  --full-page                   — Capture full scrollable page");
            println!("  --jpeg --quality <n>          — JPEG format (1-100)");
            println!("  --element <selector>          — Screenshot specific element");
            println!("  --transparent                 — Transparent background (PNG)");
            println!();
            println!("PDF options:");
            println!("  --landscape                   — Landscape orientation");
            println!("  --scale <n>                   — Scale factor (e.g. 1.5)");
            println!();
            println!("General options:");
            println!("  --proxy <url>                 — Proxy (http/socks5://host:port)");
            println!("  --header \"Key: Value\"         — Extra HTTP header (repeatable)");
            println!("  --cookie \"name=value\"         — Cookie (repeatable)");
            println!();
            println!("Examples:");
            println!("  cdp_rs js eval \"document.title\" https://example.com");
            println!("  cdp_rs js console https://example.com");
            println!("  cdp_rs js run cookies https://site.com");
            println!("  cdp_rs js inject \"console.log('hi')\" https://site.com");
            println!("  cdp_rs network log https://site.com traffic.har");
            println!("  cdp_rs network monitor https://api.com");
            println!("  cdp_rs network block \"*ads*\" https://site.com");
            println!("  cdp_rs cookies export https://site.com cookies.json");
            println!("  cdp_rs screenshot https://site.com page.png --full-page");
            println!("  cdp_rs pdf https://site.com doc.pdf --landscape");
        }
        _ => {
            // Default: launch browser
            let port: u16 = command.parse().unwrap_or(0);
            println!("CDP-RS: Browser Automation Framework (Rust)");
            println!("Launching Chrome on port {}...", if port == 0 { "random".to_string() } else { port.to_string() });

            let mut bm = BrowserManager::new().binary(CHROME).port(port);
            if let Some(ref p) = proxy { bm = bm.proxy(p); }
            if !headers.is_empty() { bm = bm.headers(headers); }
            if !cookies.is_empty() { bm = bm.cookies(cookies); }
            let browser = bm.launch().await?;

            println!("Browser launched on port {}", browser.get_port());
            println!("Press Ctrl+C to exit.");

            tokio::signal::ctrl_c().await?;
            println!("Shutting down...");
        }
    }

    Ok(())
}
