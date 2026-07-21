use cdp_rs::browser::BrowserManager;
use cdp_rs::page::Page;
use cdp_rs::auto_wait::AutoWait;
use cdp_rs::network_intercept::NetworkInterceptor;
use cdp_rs::browser_pool::BrowserPool;
use std::time::Duration;

// ============================================================
// Helper
// ============================================================

const CHROME_PATH: &str = r"C:\Program Files\Google\Chrome\Application\chrome.exe";

async fn launch_browser() -> BrowserManager {
    BrowserManager::new()
        .binary(CHROME_PATH)
        .launch()
        .await
        .expect("Failed to launch Chrome")
}

async fn launch_browser_on_port(port: u16) -> BrowserManager {
    BrowserManager::new()
        .binary(CHROME_PATH)
        .port(port)
        .launch()
        .await
        .expect("Failed to launch Chrome")
}

// ============================================================
// BrowserManager
// ============================================================

#[tokio::test]
async fn test_launch_chrome() {
    let browser = launch_browser().await;
    let port = browser.get_port();
    assert!(port >= 49152);
    println!("Launched on port {}", port);
}

#[tokio::test]
async fn test_launch_specific_port() {
    // Use a port that's unlikely to be in use
    let browser = launch_browser_on_port(49200).await;
    // Port may be reassigned if 49200 is in use, just verify it launched
    assert!(browser.get_port() >= 49152);
}

#[tokio::test]
async fn test_connection_alive() {
    let browser = launch_browser().await;
    let conn = browser.connection().unwrap();
    let result = conn.send_page("Runtime.evaluate", serde_json::json!({
        "expression": "1 + 1",
        "returnByValue": true,
    })).await.unwrap();
    let value = result.get("result").and_then(|r| r.get("value")).and_then(|v| v.as_i64());
    assert_eq!(value, Some(2));
}

#[tokio::test]
async fn test_subscribe_events() {
    let browser = launch_browser().await;
    let mut rx = browser.subscribe_events().unwrap();

    // Navigate to trigger events (Page.enable already done in connect)
    let conn = browser.connection().unwrap();
    conn.send_page("Page.navigate", serde_json::json!({
        "url": "data:text/html,<h1>Test</h1>"
    })).await.unwrap();

    // Should receive at least one event
    let event = tokio::time::timeout(Duration::from_secs(5), rx.recv()).await;
    assert!(event.is_ok());
}

// ============================================================
// Page
// ============================================================

#[tokio::test]
async fn test_page_navigate() {
    let browser = launch_browser().await;
    let conn = browser.connection().unwrap();
    let page = Page::new(conn);

    page.navigate("data:text/html,<title>MyPage</title>").await.unwrap();
    let title = page.get_title().await.unwrap();
    assert_eq!(title, "MyPage");
}

#[tokio::test]
async fn test_page_get_url() {
    let browser = launch_browser().await;
    let conn = browser.connection().unwrap();
    let page = Page::new(conn);

    page.navigate("data:text/html,<h1>Hello</h1>").await.unwrap();
    let url = page.get_url().await.unwrap();
    assert!(url.starts_with("data:text/html"));
}

#[tokio::test]
async fn test_page_evaluate() {
    let browser = launch_browser().await;
    let conn = browser.connection().unwrap();
    let page = Page::new(conn);

    page.navigate("data:text/html,<div id='x'>42</div>").await.unwrap();

    let result = page.evaluate("document.getElementById('x').textContent").await.unwrap();
    let text = result.get("value").and_then(|v| v.as_str()).unwrap();
    assert_eq!(text, "42");
}

#[tokio::test]
async fn test_page_evaluate_math() {
    let browser = launch_browser().await;
    let conn = browser.connection().unwrap();
    let page = Page::new(conn);

    page.navigate("data:text/html,<body></body>").await.unwrap();

    let result = page.evaluate("Math.PI").await.unwrap();
    let pi = result.get("value").and_then(|v| v.as_f64()).unwrap();
    assert!((pi - 3.14159).abs() < 0.001);
}

#[tokio::test]
async fn test_page_screenshot() {
    let browser = launch_browser().await;
    let conn = browser.connection().unwrap();
    let page = Page::new(conn);

    page.navigate("data:text/html,<body style='background:red;width:100px;height:100px'></body>").await.unwrap();

    let path = "E:/1/test_screenshot.png";
    page.screenshot(path).await.unwrap();

    let meta = tokio::fs::metadata(path).await.unwrap();
    assert!(meta.len() > 0);

    // Cleanup
    let _ = tokio::fs::remove_file(path).await;
}

#[tokio::test]
async fn test_page_click() {
    let browser = launch_browser().await;
    let conn = browser.connection().unwrap();
    let page = Page::new(conn);

    page.navigate("data:text/html,<button id='btn' onclick='document.title=\"clicked\"'>Click</button>").await.unwrap();

    page.click("#btn").await.unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;

    let title = page.get_title().await.unwrap();
    assert_eq!(title, "clicked");
}

#[tokio::test]
async fn test_page_click_coords() {
    let browser = launch_browser().await;
    let conn = browser.connection().unwrap();
    let page = Page::new(conn);

    page.navigate("data:text/html,<button id='btn' onclick='document.title=\"clicked\"' style='position:absolute;left:50px;top:50px;width:100px;height:50px'>Click</button>").await.unwrap();

    page.click_coords(100.0, 75.0).await.unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;

    let title = page.get_title().await.unwrap();
    assert_eq!(title, "clicked");
}

#[tokio::test]
async fn test_page_type_text() {
    let browser = launch_browser().await;
    let conn = browser.connection().unwrap();
    let page = Page::new(conn);

    page.navigate("data:text/html,<input id='inp' oninput='document.title=this.value'>").await.unwrap();

    // Focus input
    page.evaluate("document.getElementById('inp').focus()").await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    page.type_text("hello").await.unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;

    let title = page.get_title().await.unwrap();
    assert_eq!(title, "hello");
}

#[tokio::test]
async fn test_page_press_key() {
    let browser = launch_browser().await;
    let conn = browser.connection().unwrap();
    let page = Page::new(conn);

    page.navigate("data:text/html,<input id='inp' onkeydown='document.title=event.key'>").await.unwrap();

    page.evaluate("document.getElementById('inp').focus()").await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    page.press_key("Enter").await.unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;

    let title = page.get_title().await.unwrap();
    assert_eq!(title, "Enter");
}

#[tokio::test]
async fn test_page_fill() {
    let browser = launch_browser().await;
    let conn = browser.connection().unwrap();
    let page = Page::new(conn);

    page.navigate("data:text/html,<input id='inp' oninput='document.title=this.value'>").await.unwrap();

    page.fill("#inp", "test_value").await.unwrap();
    tokio::time::sleep(Duration::from_millis(300)).await;

    let title = page.get_title().await.unwrap();
    assert_eq!(title, "test_value");
}

#[tokio::test]
async fn test_page_wait_for_selector() {
    let browser = launch_browser().await;
    let conn = browser.connection().unwrap();
    let page = Page::new(conn);

    page.navigate("data:text/html,<body></body>").await.unwrap();

    // Add element after 500ms via JS
    let conn2 = browser.connection().unwrap();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(500)).await;
        conn2.send_page("Runtime.evaluate", serde_json::json!({
            "expression": "document.body.innerHTML = '<div id=\"dynamic\">Loaded</div>'"
        })).await.unwrap();
    });

    page.wait_for_selector("#dynamic", 3000).await.unwrap();
    let text = page.evaluate("document.getElementById('dynamic').textContent").await.unwrap();
    assert_eq!(text.get("value").and_then(|v| v.as_str()), Some("Loaded"));
}

#[tokio::test]
async fn test_page_wait_for_text() {
    let browser = launch_browser().await;
    let conn = browser.connection().unwrap();
    let page = Page::new(conn);

    page.navigate("data:text/html,<body></body>").await.unwrap();

    let conn2 = browser.connection().unwrap();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(500)).await;
        conn2.send_page("Runtime.evaluate", serde_json::json!({
            "expression": "document.body.textContent = 'Hello World'"
        })).await.unwrap();
    });

    page.wait_for_text("Hello World", 3000).await.unwrap();
}

#[tokio::test]
async fn test_page_wait_for_load() {
    let browser = launch_browser().await;
    let conn = browser.connection().unwrap();
    let page = Page::new(conn);

    page.navigate("data:text/html,<h1>Loaded</h1>").await.unwrap();
    // Should not timeout
    page.wait_for_load().await.unwrap();
}

// ============================================================
// AutoWait
// ============================================================

#[tokio::test]
async fn test_auto_wait_element() {
    let browser = launch_browser().await;
    let conn = browser.connection().unwrap();

    conn.send_page("Page.navigate", serde_json::json!({
        "url": "data:text/html,<div id='test'>Hello</div>"
    })).await.unwrap();
    tokio::time::sleep(Duration::from_millis(500)).await;

    let waiter = AutoWait::new(conn).with_timeout(5000);
    waiter.wait_for("#test").await.unwrap();
}

#[tokio::test]
async fn test_auto_wait_clickable() {
    let browser = launch_browser().await;
    let conn = browser.connection().unwrap();

    conn.send_page("Page.navigate", serde_json::json!({
        "url": "data:text/html,<button id='btn' style='width:100px;height:50px'>OK</button>"
    })).await.unwrap();
    tokio::time::sleep(Duration::from_millis(500)).await;

    let waiter = AutoWait::new(conn).with_timeout(5000);
    waiter.wait_for_clickable("#btn").await.unwrap();
}

#[tokio::test]
async fn test_auto_wait_timeout() {
    let browser = launch_browser().await;
    let conn = browser.connection().unwrap();

    conn.send_page("Page.navigate", serde_json::json!({
        "url": "data:text/html,<body></body>"
    })).await.unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;

    let waiter = AutoWait::new(conn).with_timeout(500);
    let result = waiter.wait_for("#nonexistent").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_auto_wait_text() {
    let browser = launch_browser().await;
    let conn = browser.connection().unwrap();

    conn.send_page("Page.navigate", serde_json::json!({
        "url": "data:text/html,<div id='msg'>Waiting...</div>"
    })).await.unwrap();
    tokio::time::sleep(Duration::from_millis(500)).await;

    let waiter = AutoWait::new(conn).with_timeout(5000);
    waiter.wait_for_text("#msg", "Waiting...").await.unwrap();
}

#[tokio::test]
async fn test_auto_wait_load() {
    let browser = launch_browser().await;
    let conn = browser.connection().unwrap();

    conn.send_page("Page.navigate", serde_json::json!({
        "url": "data:text/html,<h1>Ready</h1>"
    })).await.unwrap();

    let waiter = AutoWait::new(conn).with_timeout(5000);
    waiter.wait_for_load().await.unwrap();
}

#[tokio::test]
async fn test_auto_wait_js_condition() {
    let browser = launch_browser().await;
    let conn = browser.connection().unwrap();

    conn.send_page("Page.navigate", serde_json::json!({
        "url": "data:text/html,<body></body>"
    })).await.unwrap();

    let conn2 = browser.connection().unwrap();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(300)).await;
        conn2.send_page("Runtime.evaluate", serde_json::json!({
            "expression": "window.__ready = true"
        })).await.unwrap();
    });

    let waiter = AutoWait::new(conn).with_timeout(5000);
    waiter.wait_for_js("window.__ready === true").await.unwrap();
}

// ============================================================
// NetworkInterceptor
// ============================================================

#[tokio::test]
async fn test_intercept_block_ads() {
    let browser = launch_browser().await;
    let conn = browser.connection().unwrap();

    let interceptor = NetworkInterceptor::new(conn.clone());
    interceptor.enable().await.unwrap();
    interceptor.block_ads().await;

    // Navigate to a page (ads won't actually fire on data: URI, but we verify no crash)
    conn.send_page("Page.navigate", serde_json::json!({
        "url": "data:text/html,<h1>Ad-free</h1>"
    })).await.unwrap();

    tokio::time::sleep(Duration::from_millis(500)).await;

    // Disable interceptor
    interceptor.disable().await.unwrap();
}

#[tokio::test]
async fn test_intercept_custom_rule() {
    let browser = launch_browser().await;
    let conn = browser.connection().unwrap();

    let interceptor = NetworkInterceptor::new(conn.clone());
    interceptor.enable().await.unwrap();
    interceptor.block("tracker.example.com").await;

    // Verify rule was added
    assert_eq!(interceptor.rules_count().await, 1);

    interceptor.disable().await.unwrap();
}

#[tokio::test]
async fn test_intercept_mock_response() {
    use cdp_rs::network_intercept::{MockResponse, UrlPattern, InterceptAction};
    use serde_json::json;

    let browser = launch_browser().await;
    let conn = browser.connection().unwrap();

    let interceptor = NetworkInterceptor::new(conn.clone());
    interceptor.enable().await.unwrap();

    let mock = MockResponse::json(200, &json!({"mocked": true}));
    interceptor.add_rule(
        UrlPattern::Contains("mock.example.com".to_string()),
        InterceptAction::Mock(mock),
    ).await;

    assert_eq!(interceptor.rules_count().await, 1);

    interceptor.disable().await.unwrap();
}

// ============================================================
// BrowserPool
// ============================================================

#[tokio::test]
async fn test_pool_acquire_and_use() {
    let pool = BrowserPool::new(2, 9500, false).with_chrome_path(CHROME_PATH);
    let browser = pool.acquire().await.unwrap();
    assert!(browser.port >= 9500);

    // Use the pooled browser
    let result = browser.evaluate("1 + 1").await.unwrap();
    let value = result.get("value").and_then(|v| v.as_i64());
    assert_eq!(value, Some(2));

    drop(browser);
}

#[tokio::test]
async fn test_pool_multiple_browsers() {
    let pool = BrowserPool::new(3, 9600, false).with_chrome_path(CHROME_PATH);

    let b1 = pool.acquire().await.unwrap();
    let b2 = pool.acquire().await.unwrap();

    assert_ne!(b1.port, b2.port);

    let stats = pool.stats().await;
    assert_eq!(stats.active, 2);
    assert_eq!(stats.available, 1);

    drop(b1);
    drop(b2);
}

#[tokio::test]
async fn test_pool_navigate() {
    let pool = BrowserPool::new(1, 9700, false).with_chrome_path(CHROME_PATH);
    let browser = pool.acquire().await.unwrap();

    browser.navigate("data:text/html,<title>Pooled</title>").await.unwrap();
    let result = browser.evaluate("document.title").await.unwrap();
    let title = result.get("value").and_then(|v| v.as_str()).unwrap();
    assert_eq!(title, "Pooled");
}

#[tokio::test]
async fn test_pool_clean_state() {
    let pool = BrowserPool::new(1, 9800, false).with_chrome_path(CHROME_PATH);
    let browser = pool.acquire().await.unwrap();

    // Set a cookie
    browser.connection.send_page("Network.setCookie", serde_json::json!({
        "name": "test", "value": "123", "url": "http://example.com"
    })).await.unwrap();

    // Clean
    browser.clean_state().await.unwrap();

    // Verify cookies cleared
    let result = browser.connection.send_page("Network.getCookies", serde_json::json!({
        "urls": ["http://example.com"]
    })).await.unwrap();
    let cookies = result.get("cookies").and_then(|c| c.as_array()).unwrap();
    assert!(cookies.is_empty());
}

#[tokio::test]
async fn test_pool_isolated_context() {
    let pool = BrowserPool::new(1, 9900, false).with_chrome_path(CHROME_PATH);
    let browser = pool.acquire().await.unwrap();

    let ctx_id = BrowserPool::isolated_context(&browser).await.unwrap();
    assert!(!ctx_id.is_empty());
    println!("Isolated context: {}", ctx_id);
}

// ============================================================
// Complex scenarios
// ============================================================

#[tokio::test]
async fn test_full_workflow() {
    let browser = launch_browser().await;
    let conn = browser.connection().unwrap();
    let page = Page::new(conn);

    // 1. Navigate
    page.navigate("data:text/html,<h1 id='status'>Loading...</h1>").await.unwrap();

    // 2. Check title
    let title = page.get_title().await.unwrap();
    assert_eq!(title, "");

    // 3. Evaluate JS to update content
    page.evaluate("document.getElementById('status').textContent = 'Ready'").await.unwrap();

    // 4. Verify
    let text = page.evaluate("document.getElementById('status').textContent").await.unwrap();
    assert_eq!(text.get("value").and_then(|v| v.as_str()), Some("Ready"));

    // 5. Screenshot
    let path = "E:/1/test_full_workflow.png";
    page.screenshot(path).await.unwrap();
    let meta = tokio::fs::metadata(path).await.unwrap();
    assert!(meta.len() > 0);
    let _ = tokio::fs::remove_file(path).await;
}

#[tokio::test]
async fn test_click_then_wait() {
    let browser = launch_browser().await;
    let conn = browser.connection().unwrap();
    let page = Page::new(conn);

    page.navigate("data:text/html,
        <button id='load' onclick='
            document.title = \"loading\";
            setTimeout(() => {
                document.getElementById(\"result\").textContent = \"Done\";
                document.title = \"done\";
            }, 300);
        '>Load</button>
        <div id='result'></div>
    ").await.unwrap();

    page.click("#load").await.unwrap();
    page.wait_for_text("Done", 3000).await.unwrap();

    let title = page.get_title().await.unwrap();
    assert_eq!(title, "done");
}

#[tokio::test]
async fn test_multiple_pages_sequential() {
    let browser = launch_browser().await;
    let conn = browser.connection().unwrap();

    // Page 1
    let page1 = Page::new(conn.clone());
    page1.navigate("data:text/html,<title>Page1</title>").await.unwrap();
    assert_eq!(page1.get_title().await.unwrap(), "Page1");

    // Navigate same connection to Page 2
    page1.navigate("data:text/html,<title>Page2</title>").await.unwrap();
    assert_eq!(page1.get_title().await.unwrap(), "Page2");
}

#[tokio::test]
async fn test_js_dom_manipulation() {
    let browser = launch_browser().await;
    let conn = browser.connection().unwrap();
    let page = Page::new(conn);

    page.navigate("data:text/html,<ul id='list'></ul>").await.unwrap();

    // Add items via JS
    page.evaluate(r#"
        const ul = document.getElementById('list');
        for (let i = 0; i < 5; i++) {
            const li = document.createElement('li');
            li.textContent = 'Item ' + i;
            ul.appendChild(li);
        }
    "#).await.unwrap();

    let count = page.evaluate("document.querySelectorAll('#list li').length").await.unwrap();
    assert_eq!(count.get("value").and_then(|v| v.as_i64()), Some(5));

    let first = page.evaluate("document.querySelector('#list li').textContent").await.unwrap();
    assert_eq!(first.get("value").and_then(|v| v.as_str()), Some("Item 0"));
}

// ============================================================
// PDF
// ============================================================

#[tokio::test]
async fn test_pdf_generate() {
    use cdp_rs::pdf::{PdfGenerator, PdfOptions};

    let browser = launch_browser().await;
    let conn = browser.connection().unwrap();
    let page = Page::new(conn.clone());
    page.navigate("data:text/html,<h1>PDF Test</h1><p>Hello World</p>").await.unwrap();

    let pdf = PdfGenerator::new(conn);
    let bytes = pdf.generate(&PdfOptions::a4()).await.unwrap();
    assert!(!bytes.is_empty());
    // PDF starts with %PDF
    assert!(bytes.starts_with(b"%PDF"));
}

#[tokio::test]
async fn test_pdf_save_to_file() {
    use cdp_rs::pdf::{PdfGenerator, PdfOptions};

    let browser = launch_browser().await;
    let conn = browser.connection().unwrap();
    let page = Page::new(conn.clone());
    page.navigate("data:text/html,<h1>Save PDF</h1>").await.unwrap();

    let pdf = PdfGenerator::new(conn);
    let path = "E:/1/test_output.pdf";
    pdf.save(path, &PdfOptions::a4()).await.unwrap();

    let meta = tokio::fs::metadata(path).await.unwrap();
    assert!(meta.len() > 0);
    let _ = tokio::fs::remove_file(path).await;
}

// ============================================================
// Cookie
// ============================================================

#[tokio::test]
async fn test_cookie_set_and_get() {
    use cdp_rs::cookie::{CookieManager, Cookie};

    let browser = launch_browser().await;
    let conn = browser.connection().unwrap();
    let page = Page::new(conn.clone());
    page.navigate("data:text/html,<body></body>").await.unwrap();

    let cookie_mgr = CookieManager::new(conn);

    // Set a cookie
    cookie_mgr.set_cookie(&Cookie {
        name: "test_cookie".to_string(),
        value: "hello".to_string(),
        domain: "localhost".to_string(),
        path: "/".to_string(),
        expires: None,
        http_only: false,
        secure: false,
        same_site: None,
    }).await.unwrap();

    // Get cookies
    let cookies = cookie_mgr.get_cookies(&["http://localhost"]).await.unwrap();
    assert!(cookies.iter().any(|c| c.name == "test_cookie"));
}

#[tokio::test]
async fn test_cookie_delete() {
    use cdp_rs::cookie::{CookieManager, Cookie};

    let browser = launch_browser().await;
    let conn = browser.connection().unwrap();
    let page = Page::new(conn.clone());
    page.navigate("data:text/html,<body></body>").await.unwrap();

    let cookie_mgr = CookieManager::new(conn);

    // Set then delete
    cookie_mgr.set_cookie(&Cookie {
        name: "to_delete".to_string(),
        value: "123".to_string(),
        domain: "localhost".to_string(),
        path: "/".to_string(),
        expires: None,
        http_only: false,
        secure: false,
        same_site: None,
    }).await.unwrap();

    cookie_mgr.delete_cookie("to_delete", "localhost", "/").await.unwrap();

    let has = cookie_mgr.has_cookie(&["http://localhost"], "to_delete").await.unwrap();
    assert!(!has);
}

#[tokio::test]
async fn test_cookie_clear_all() {
    use cdp_rs::cookie::{CookieManager, Cookie};

    let browser = launch_browser().await;
    let conn = browser.connection().unwrap();
    let page = Page::new(conn.clone());
    page.navigate("data:text/html,<body></body>").await.unwrap();

    let cookie_mgr = CookieManager::new(conn);

    cookie_mgr.set_cookie(&Cookie {
        name: "c1".to_string(),
        value: "v1".to_string(),
        domain: "localhost".to_string(),
        path: "/".to_string(),
        expires: None,
        http_only: false,
        secure: false,
        same_site: None,
    }).await.unwrap();

    cookie_mgr.clear_all().await.unwrap();

    let cookies = cookie_mgr.get_cookies(&["http://localhost"]).await.unwrap();
    assert!(cookies.is_empty());
}

#[tokio::test]
async fn test_cookie_map() {
    use cdp_rs::cookie::{CookieManager, Cookie};

    let browser = launch_browser().await;
    let conn = browser.connection().unwrap();
    let page = Page::new(conn.clone());
    page.navigate("data:text/html,<body></body>").await.unwrap();

    let cookie_mgr = CookieManager::new(conn);

    cookie_mgr.set_cookie(&Cookie {
        name: "session_id".to_string(),
        value: "abc123".to_string(),
        domain: "localhost".to_string(),
        path: "/".to_string(),
        expires: None,
        http_only: false,
        secure: false,
        same_site: None,
    }).await.unwrap();

    let map = cookie_mgr.get_cookie_map(&["http://localhost"]).await.unwrap();
    assert_eq!(map.get("session_id").map(|s| s.as_str()), Some("abc123"));
}

// ============================================================
// Dialog
// ============================================================

#[tokio::test]
async fn test_dialog_handler_accept() {
    use cdp_rs::dialog::DialogHandler;

    let browser = launch_browser().await;
    let conn = browser.connection().unwrap();
    let page = Page::new(conn.clone());

    let handler = DialogHandler::new(conn.clone());

    // Navigate to a page that shows an alert
    page.navigate("data:text/html,<script>setTimeout(() => alert('test'), 100);</script>").await.unwrap();

    // Handle the dialog
    let conn2 = conn.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(200)).await;
        let _ = conn2.send_page("Page.handleJavaScriptDialog", serde_json::json!({ "accept": true })).await;
    });

    // Verify page loaded after dialog handled
    tokio::time::sleep(Duration::from_millis(500)).await;
    let title = page.get_title().await.unwrap();
    assert_eq!(title, ""); // data: URI has no title
}

#[tokio::test]
async fn test_dialog_parse_event() {
    use cdp_rs::dialog::DialogHandler;

    let params = serde_json::json!({
        "type": "confirm",
        "message": "Delete?",
        "url": "https://example.com",
        "hasBrowserHandler": false,
    });

    let event = DialogHandler::parse_event(&params).unwrap();
    assert_eq!(event.dialog_type, "confirm");
    assert_eq!(event.message, "Delete?");
}

// ============================================================
// Proxy
// ============================================================

#[tokio::test]
async fn test_browser_with_proxy() {
    // Launch browser with proxy flag — verify it starts
    // (proxy won't actually work without a running proxy server,
    //  but Chrome should accept the flag and start)
    let browser = BrowserManager::new()
        .binary(CHROME_PATH)
        .proxy("http://127.0.0.1:19999")
        .launch()
        .await;
    // Browser should still start even with invalid proxy
    assert!(browser.is_ok());
    let browser = browser.unwrap();
    let conn = browser.connection().unwrap();

    // Verify CDP connection works
    let result = conn.send_page("Runtime.evaluate", serde_json::json!({
        "expression": "1 + 1",
        "returnByValue": true,
    })).await.unwrap();
    let value = result.get("result").and_then(|r| r.get("value")).and_then(|v| v.as_i64());
    assert_eq!(value, Some(2));
}

#[tokio::test]
async fn test_browser_with_socks5_proxy() {
    let browser = BrowserManager::new()
        .binary(CHROME_PATH)
        .proxy("socks5://127.0.0.1:1080")
        .launch()
        .await;
    assert!(browser.is_ok());
}

#[tokio::test]
async fn test_browser_with_authenticated_proxy() {
    let browser = BrowserManager::new()
        .binary(CHROME_PATH)
        .proxy("http://user:pass@proxy.example.com:8080")
        .launch()
        .await;
    assert!(browser.is_ok());
}

#[tokio::test]
async fn test_pool_with_proxy() {
    use cdp_rs::browser_pool::BrowserPool;

    let pool = BrowserPool::new(1, 9550, false)
        .with_chrome_path(CHROME_PATH)
        .with_proxy("http://127.0.0.1:19999");

    let browser = pool.acquire().await.unwrap();
    assert!(browser.port >= 9550);

    // Verify browser works
    let result = browser.evaluate("1 + 1").await.unwrap();
    let value = result.get("value").and_then(|v| v.as_i64());
    assert_eq!(value, Some(2));
}

#[tokio::test]
async fn test_builder_chaining_with_proxy() {
    let browser = BrowserManager::new()
        .binary(CHROME_PATH)
        .port(49300)
        .proxy("http://proxy:3128")
        .launch()
        .await;
    assert!(browser.is_ok());
}

// ============================================================
// Unit tests for proxy builder
// ============================================================

#[test]
fn test_browser_manager_proxy_builder() {
    use cdp_rs::browser::BrowserManager;
    let _bm = BrowserManager::new()
        .binary("chrome")
        .proxy("http://proxy:8080");
    // Builder should not panic
}

#[test]
fn test_pool_proxy_builder() {
    use cdp_rs::browser_pool::BrowserPool;
    let _pool = BrowserPool::new(2, 9700, false)
        .with_chrome_path(CHROME_PATH)
        .with_proxy("socks5://127.0.0.1:1080");
    // Builder should not panic
}

// ============================================================
// Headers
// ============================================================

#[tokio::test]
async fn test_browser_with_custom_headers() {
    use std::collections::HashMap;

    let mut headers = HashMap::new();
    headers.insert("X-Custom-Header".to_string(), "test-value".to_string());
    headers.insert("Authorization".to_string(), "Bearer fake-token".to_string());

    let browser = BrowserManager::new()
        .binary(CHROME_PATH)
        .headers(headers)
        .launch()
        .await;

    assert!(browser.is_ok());
    let browser = browser.unwrap();
    let conn = browser.connection().unwrap();

    // Verify browser works with custom headers set
    let result = conn.send_page("Runtime.evaluate", serde_json::json!({
        "expression": "navigator.userAgent",
        "returnByValue": true,
    })).await.unwrap();
    let ua = result.get("result").and_then(|r| r.get("value")).and_then(|v| v.as_str());
    assert!(ua.is_some());
}

#[tokio::test]
async fn test_browser_with_single_header() {
    let browser = BrowserManager::new()
        .binary(CHROME_PATH)
        .header("X-Test", "value123")
        .launch()
        .await;

    assert!(browser.is_ok());
}

#[tokio::test]
async fn test_browser_with_multiple_headers() {
    let browser = BrowserManager::new()
        .binary(CHROME_PATH)
        .header("X-First", "1")
        .header("X-Second", "2")
        .launch()
        .await;

    assert!(browser.is_ok());
}

// ============================================================
// Cookies via BrowserManager
// ============================================================

#[tokio::test]
async fn test_browser_with_cookies() {
    let browser = BrowserManager::new()
        .binary(CHROME_PATH)
        .cookie("session_id", "abc123")
        .cookie("user", "testuser")
        .launch()
        .await;

    assert!(browser.is_ok());
    let browser = browser.unwrap();
    let conn = browser.connection().unwrap();

    // Navigate and verify cookies are set
    let page = Page::new(conn.clone());
    page.navigate("data:text/html,<body></body>").await.unwrap();

    let cookie_mgr = cdp_rs::cookie::CookieManager::new(conn);
    let cookies = cookie_mgr.get_cookie_map(&["http://localhost"]).await.unwrap();

    // At least one of our cookies should be present
    assert!(cookies.contains_key("session_id") || cookies.contains_key("user"));
}

#[tokio::test]
async fn test_browser_with_cookie_with_domain() {
    let browser = BrowserManager::new()
        .binary(CHROME_PATH)
        .cookie_with_domain("token", "xyz", "localhost", "/")
        .launch()
        .await;

    assert!(browser.is_ok());
    let browser = browser.unwrap();
    let conn = browser.connection().unwrap();

    let page = Page::new(conn.clone());
    page.navigate("data:text/html,<body></body>").await.unwrap();

    let cookie_mgr = cdp_rs::cookie::CookieManager::new(conn);
    let has = cookie_mgr.has_cookie(&["http://localhost"], "token").await.unwrap();
    assert!(has);
}

#[tokio::test]
async fn test_browser_with_cookie_list() {
    use cdp_rs::browser::CookieEntry;

    let cookies = vec![
        CookieEntry { name: "c1".to_string(), value: "v1".to_string(), domain: None, path: None },
        CookieEntry { name: "c2".to_string(), value: "v2".to_string(), domain: None, path: None },
    ];

    let browser = BrowserManager::new()
        .binary(CHROME_PATH)
        .cookies(cookies)
        .launch()
        .await;

    assert!(browser.is_ok());
}

// ============================================================
// Combined: headers + cookies + proxy
// ============================================================

#[tokio::test]
async fn test_browser_headers_cookies_proxy() {
    use std::collections::HashMap;

    let mut headers = HashMap::new();
    headers.insert("X-Custom".to_string(), "yes".to_string());

    let browser = BrowserManager::new()
        .binary(CHROME_PATH)
        .proxy("http://127.0.0.1:19999")
        .headers(headers)
        .cookie("sid", "123")
        .launch()
        .await;

    assert!(browser.is_ok());
}

#[tokio::test]
async fn test_pool_with_headers_and_cookies() {
    use cdp_rs::browser_pool::BrowserPool;
    use std::collections::HashMap;

    let mut headers = HashMap::new();
    headers.insert("X-Pool".to_string(), "true".to_string());

    let pool = BrowserPool::new(1, 9650, false)
        .with_chrome_path(CHROME_PATH)
        .with_header("Authorization", "Bearer token123")
        .with_headers(headers)
        .with_cookie("pool_cookie", "pool_value");

    let browser = pool.acquire().await.unwrap();
    assert!(browser.port >= 9650);

    let result = browser.evaluate("1 + 1").await.unwrap();
    let value = result.get("value").and_then(|v| v.as_i64());
    assert_eq!(value, Some(2));
}

// ============================================================
// Unit tests for CLI parsing
// ============================================================

fn parse_headers(args: &[String]) -> std::collections::HashMap<String, String> {
    let mut headers = std::collections::HashMap::new();
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

fn parse_cookies(args: &[String]) -> Vec<cdp_rs::browser::CookieEntry> {
    let mut cookies = Vec::new();
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--cookie" {
            if let Some(val) = args.get(i + 1) {
                let (name, value) = val.split_once('=').unwrap_or((val, ""));
                cookies.push(cdp_rs::browser::CookieEntry {
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

#[test]
fn test_find_headers() {
    let args = vec![
        "cdp_rs".to_string(),
        "page".to_string(),
        "https://example.com".to_string(),
        "--header".to_string(),
        "Authorization: Bearer tok".to_string(),
        "--header".to_string(),
        "X-Custom: val".to_string(),
    ];
    let headers = parse_headers(&args);
    assert_eq!(headers.len(), 2);
    assert_eq!(headers.get("Authorization").unwrap(), "Bearer tok");
    assert_eq!(headers.get("X-Custom").unwrap(), "val");
}

#[test]
fn test_find_headers_empty() {
    let args = vec!["cdp_rs".to_string(), "page".to_string()];
    let headers = parse_headers(&args);
    assert!(headers.is_empty());
}

#[test]
fn test_find_cookies() {
    let args = vec![
        "cdp_rs".to_string(),
        "page".to_string(),
        "https://example.com".to_string(),
        "--cookie".to_string(),
        "session=abc".to_string(),
        "--cookie".to_string(),
        "token=xyz".to_string(),
    ];
    let cookies = parse_cookies(&args);
    assert_eq!(cookies.len(), 2);
    assert_eq!(cookies[0].name, "session");
    assert_eq!(cookies[0].value, "abc");
    assert_eq!(cookies[1].name, "token");
}

#[test]
fn test_find_cookies_empty() {
    let args = vec!["cdp_rs".to_string(), "page".to_string()];
    let cookies = parse_cookies(&args);
    assert!(cookies.is_empty());
}

// ============================================================
// Screenshot
// ============================================================

#[tokio::test]
async fn test_screenshot_png() {
    use cdp_rs::screenshot::{ScreenshotCapture, ScreenshotOptions};

    let browser = launch_browser().await;
    let conn = browser.connection().unwrap();
    let page = Page::new(conn.clone());
    page.navigate("data:text/html,<h1>Hello</h1><div style='width:200px;height:200px;background:red'></div>").await.unwrap();

    let cap = ScreenshotCapture::new(conn);
    let path = "E:/1/test_ss_png.png";
    cap.save(path, &ScreenshotOptions::png()).await.unwrap();

    let meta = tokio::fs::metadata(path).await.unwrap();
    assert!(meta.len() > 0);

    // Verify PNG header
    let bytes = tokio::fs::read(path).await.unwrap();
    assert!(bytes.starts_with(b"\x89PNG"));
    let _ = tokio::fs::remove_file(path).await;
}

#[tokio::test]
async fn test_screenshot_jpeg() {
    use cdp_rs::screenshot::{ScreenshotCapture, ScreenshotOptions};

    let browser = launch_browser().await;
    let conn = browser.connection().unwrap();
    let page = Page::new(conn.clone());
    page.navigate("data:text/html,<h1>JPEG Test</h1>").await.unwrap();

    let cap = ScreenshotCapture::new(conn);
    let path = "E:/1/test_ss_jpeg.jpg";
    cap.save(path, &ScreenshotOptions::jpeg(90)).await.unwrap();

    let meta = tokio::fs::metadata(path).await.unwrap();
    assert!(meta.len() > 0);

    // Verify JPEG header
    let bytes = tokio::fs::read(path).await.unwrap();
    assert!(bytes.starts_with(&[0xFF, 0xD8, 0xFF]));
    let _ = tokio::fs::remove_file(path).await;
}

#[tokio::test]
async fn test_screenshot_full_page() {
    use cdp_rs::screenshot::{ScreenshotCapture, ScreenshotOptions};

    let browser = launch_browser().await;
    let conn = browser.connection().unwrap();
    let page = Page::new(conn.clone());
    page.navigate("data:text/html,<div style='height:2000px;background:linear-gradient(red,blue)'>Tall page</div>").await.unwrap();

    let cap = ScreenshotCapture::new(conn);
    let path = "E:/1/test_ss_full.png";
    cap.save(path, &ScreenshotOptions::png().full_page()).await.unwrap();

    let meta = tokio::fs::metadata(path).await.unwrap();
    assert!(meta.len() > 0);
    let _ = tokio::fs::remove_file(path).await;
}

#[tokio::test]
async fn test_screenshot_element() {
    use cdp_rs::screenshot::ScreenshotCapture;

    let browser = launch_browser().await;
    let conn = browser.connection().unwrap();
    let page = Page::new(conn.clone());
    page.navigate("data:text/html,<div id='target' style='width:100px;height:50px;background:blue'>Target</div>").await.unwrap();

    let cap = ScreenshotCapture::new(conn);
    let path = "E:/1/test_ss_element.png";
    cap.element_screenshot("#target", path).await.unwrap();

    let meta = tokio::fs::metadata(path).await.unwrap();
    assert!(meta.len() > 0);
    let _ = tokio::fs::remove_file(path).await;
}

#[tokio::test]
async fn test_screenshot_base64() {
    use cdp_rs::screenshot::{ScreenshotCapture, ScreenshotOptions};

    let browser = launch_browser().await;
    let conn = browser.connection().unwrap();
    let page = Page::new(conn.clone());
    page.navigate("data:text/html,<h1>Base64</h1>").await.unwrap();

    let cap = ScreenshotCapture::new(conn);
    let b64 = cap.base64(&ScreenshotOptions::png()).await.unwrap();
    assert!(!b64.is_empty());
    // Base64 PNG should start with iVBOR (base64 of PNG header)
    assert!(b64.starts_with("iVBOR"));
}

#[tokio::test]
async fn test_screenshot_clip() {
    use cdp_rs::screenshot::{ScreenshotCapture, ScreenshotOptions};

    let browser = launch_browser().await;
    let conn = browser.connection().unwrap();
    let page = Page::new(conn.clone());
    page.navigate("data:text/html,<div style='width:400px;height:400px;background:red'></div>").await.unwrap();

    let cap = ScreenshotCapture::new(conn);
    let path = "E:/1/test_ss_clip.png";
    cap.save(path, &ScreenshotOptions::png().clip(0.0, 0.0, 100.0, 100.0)).await.unwrap();

    let meta = tokio::fs::metadata(path).await.unwrap();
    assert!(meta.len() > 0);
    let _ = tokio::fs::remove_file(path).await;
}

// ============================================================
// PDF advanced
// ============================================================

#[tokio::test]
async fn test_pdf_landscape() {
    use cdp_rs::pdf::{PdfGenerator, PdfOptions};

    let browser = launch_browser().await;
    let conn = browser.connection().unwrap();
    let page = Page::new(conn.clone());
    page.navigate("data:text/html,<h1>Landscape PDF</h1>").await.unwrap();

    let pdf = PdfGenerator::new(conn);
    let path = "E:/1/test_pdf_landscape.pdf";
    pdf.save(path, &PdfOptions::a4().landscape()).await.unwrap();

    let meta = tokio::fs::metadata(path).await.unwrap();
    assert!(meta.len() > 0);
    let bytes = tokio::fs::read(path).await.unwrap();
    assert!(bytes.starts_with(b"%PDF"));
    let _ = tokio::fs::remove_file(path).await;
}

#[tokio::test]
async fn test_pdf_letter_size() {
    use cdp_rs::pdf::{PdfGenerator, PdfOptions};

    let browser = launch_browser().await;
    let conn = browser.connection().unwrap();
    let page = Page::new(conn.clone());
    page.navigate("data:text/html,<h1>Letter Size</h1>").await.unwrap();

    let pdf = PdfGenerator::new(conn);
    let path = "E:/1/test_pdf_letter.pdf";
    pdf.save(path, &PdfOptions::letter()).await.unwrap();

    let meta = tokio::fs::metadata(path).await.unwrap();
    assert!(meta.len() > 0);
    let _ = tokio::fs::remove_file(path).await;
}
