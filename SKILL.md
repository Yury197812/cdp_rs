# cdp_rs

## For User
Rust фреймворк для автоматизации браузера через CDP (Chrome DevTools Protocol).
Замена Python скриптов (gmail_oauth_auto.py, cdp_automate.py).

## For LLM
Invoke: `cdp_rs.exe <command> [options]`

Commands:
- `gmail-oauth auth` — Gmail OAuth auto-setup (replace gmail_oauth_auto.py)
- `intercept` — Browse with ad-blocker (replace cdp_automate.py)
- `pool [size]` — Browser pool for parallel testing
- `page <url> [screenshot]` — Open URL, take optional screenshot
- `help` — Show help

Modules:
- `browser.rs` — BrowserManager + CdpConnection (HTTP + WebSocket)
- `cdp_client.rs` — Raw WebSocket CDP client with event bus
- `page.rs` — High-level Page API (navigate, click, type, fill, screenshot, wait_for_*)
- `gmail_oauth.rs` — Gmail OAuth automation (Rust port of gmail_oauth_auto.py)
- `auto_wait.rs` — Event-driven element waiting
- `network_intercept.rs` — Request interception + ad-blocker
- `browser_pool.rs` — Multi-browser pool for parallel testing

Dependencies: tokio, serde_json, reqwest, anyhow, native-tls, url, dirs, futures-util

Security:
- Random ports prevent port conflicts
- Localhost binding prevents remote access
- Drop trait ensures cleanup on panic

Performance:
- Binary: 5 MB (Rust, optimized)
- Memory: ~50 MB baseline
- Startup: ~2s (Chrome launch + CDP handshake)
