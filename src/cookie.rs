use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use crate::browser::CdpConnection;

/// Cookie structure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Cookie {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    pub expires: Option<f64>,
    #[serde(rename = "httpOnly", alias = "http_only", default)]
    pub http_only: bool,
    #[serde(default)]
    pub secure: bool,
    #[serde(rename = "sameSite", alias = "same_site")]
    pub same_site: Option<String>,
}

impl Cookie {
    /// Create a simple cookie with defaults
    pub fn new(name: &str, value: &str) -> Self {
        Self {
            name: name.to_string(),
            value: value.to_string(),
            domain: String::new(),
            path: "/".to_string(),
            expires: None,
            http_only: false,
            secure: false,
            same_site: None,
        }
    }

    /// Set domain
    pub fn domain(mut self, domain: &str) -> Self {
        self.domain = domain.to_string();
        self
    }

    /// Set path
    pub fn path(mut self, path: &str) -> Self {
        self.path = path.to_string();
        self
    }

    /// Set expiration (unix timestamp)
    pub fn expires(mut self, ts: f64) -> Self {
        self.expires = Some(ts);
        self
    }

    /// Set httpOnly
    pub fn http_only(mut self, v: bool) -> Self {
        self.http_only = v;
        self
    }

    /// Set secure
    pub fn secure(mut self, v: bool) -> Self {
        self.secure = v;
        self
    }

    /// Set sameSite
    pub fn same_site(mut self, v: &str) -> Self {
        self.same_site = Some(v.to_string());
        self
    }

    /// Check if cookie is expired
    pub fn is_expired(&self) -> bool {
        self.expires
            .map(|e| e < chrono::Utc::now().timestamp() as f64)
            .unwrap_or(false)
    }

    /// Check if cookie matches a domain (handles leading dot)
    pub fn domain_matches(&self, domain: &str) -> bool {
        if self.domain == domain {
            return true;
        }
        if self.domain.starts_with('.') {
            domain.ends_with(&self.domain) || domain == &self.domain[1..]
        } else {
            domain == self.domain
        }
    }

    /// Generate Set-Cookie header value
    pub fn to_set_cookie_header(&self) -> String {
        let mut parts = vec![format!("{}={}", self.name, self.value)];
        if !self.domain.is_empty() {
            parts.push(format!("Domain={}", self.domain));
        }
        if self.path != "/" {
            parts.push(format!("Path={}", self.path));
        }
        if let Some(expires) = self.expires {
            parts.push(format!("Expires={}", expires as u64));
        }
        if self.http_only {
            parts.push("HttpOnly".to_string());
        }
        if self.secure {
            parts.push("Secure".to_string());
        }
        if let Some(ref ss) = self.same_site {
            parts.push(format!("SameSite={}", ss));
        }
        parts.join("; ")
    }

    /// Generate Cookie header value (for HTTP requests)
    pub fn to_cookie_header(&self) -> String {
        format!("{}={}", self.name, self.value)
    }
}

/// Standalone cookie jar for offline management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CookieJar {
    cookies: Vec<Cookie>,
}

impl CookieJar {
    pub fn new() -> Self {
        Self {
            cookies: Vec::new(),
        }
    }

    /// Add a cookie to the jar
    pub fn add(&mut self, cookie: Cookie) {
        self.cookies.retain(|c| {
            !(c.name == cookie.name && c.domain == cookie.domain && c.path == cookie.path)
        });
        self.cookies.push(cookie);
    }

    /// Add multiple cookies
    pub fn add_many(&mut self, cookies: Vec<Cookie>) {
        for c in cookies {
            self.add(c);
        }
    }

    /// Remove a cookie by name/domain/path
    pub fn remove(&mut self, name: &str, domain: &str, path: &str) -> bool {
        let before = self.cookies.len();
        self.cookies.retain(|c| {
            !(c.name == name && c.domain == domain && c.path == path)
        });
        self.cookies.len() < before
    }

    /// Remove all cookies matching a name pattern
    pub fn remove_by_name(&mut self, name: &str) -> usize {
        let before = self.cookies.len();
        self.cookies.retain(|c| c.name != name);
        before - self.cookies.len()
    }

    /// Remove all cookies for a domain
    pub fn remove_by_domain(&mut self, domain: &str) -> usize {
        let before = self.cookies.len();
        self.cookies.retain(|c| !c.domain_matches(domain));
        before - self.cookies.len()
    }

    /// Remove expired cookies
    pub fn remove_expired(&mut self) -> usize {
        let before = self.cookies.len();
        self.cookies.retain(|c| !c.is_expired());
        before - self.cookies.len()
    }

    /// Clear all cookies
    pub fn clear(&mut self) {
        self.cookies.clear();
    }

    /// Get all cookies
    pub fn all(&self) -> &[Cookie] {
        &self.cookies
    }

    /// Get cookies matching a domain
    pub fn for_domain(&self, domain: &str) -> Vec<&Cookie> {
        self.cookies.iter().filter(|c| c.domain_matches(domain)).collect()
    }

    /// Get a cookie by name
    pub fn get(&self, name: &str) -> Option<&Cookie> {
        self.cookies.iter().find(|c| c.name == name)
    }

    /// Get cookie value by name
    pub fn get_value(&self, name: &str) -> Option<&str> {
        self.get(name).map(|c| c.value.as_str())
    }

    /// Get cookies as HashMap
    pub fn to_map(&self) -> HashMap<String, String> {
        self.cookies.iter()
            .map(|c| (c.name.clone(), c.value.clone()))
            .collect()
    }

    /// Generate Cookie header string for HTTP requests
    pub fn to_cookie_header(&self) -> String {
        self.cookies
            .iter()
            .map(|c| c.to_cookie_header())
            .collect::<Vec<_>>()
            .join("; ")
    }

    /// Generate Set-Cookie headers
    pub fn to_set_cookie_headers(&self) -> Vec<String> {
        self.cookies.iter().map(|c| c.to_set_cookie_header()).collect()
    }

    /// Filter cookies by predicate
    pub fn filter<F>(&self, f: F) -> Vec<&Cookie>
    where
        F: Fn(&Cookie) -> bool,
    {
        self.cookies.iter().filter(|c| f(c)).collect()
    }

    /// Count cookies
    pub fn len(&self) -> usize {
        self.cookies.len()
    }

    /// Check if jar is empty
    pub fn is_empty(&self) -> bool {
        self.cookies.is_empty()
    }

    /// Merge another jar into this one
    pub fn merge(&mut self, other: &CookieJar) {
        for cookie in &other.cookies {
            self.add(cookie.clone());
        }
    }

    /// Export to JSON file
    pub async fn save_json(&self, path: &str) -> Result<()> {
        let json = serde_json::to_string_pretty(&self.cookies)?;
        tokio::fs::write(path, json).await?;
        Ok(())
    }

    /// Import from JSON file
    pub async fn load_json(path: &str) -> Result<Self> {
        let content = tokio::fs::read_to_string(path).await?;
        let cookies: Vec<Cookie> = serde_json::from_str(&content)?;
        Ok(Self { cookies })
    }

    /// Export to Netscape cookies.txt format
    pub fn to_netscape(&self) -> String {
        let mut lines = vec![
            "# Netscape HTTP Cookie File".to_string(),
            "# https://curl.haxx.se/rfc/cookie_spec.html".to_string(),
            "".to_string(),
        ];
        for c in &self.cookies {
            let domain = if c.domain.starts_with('.') {
                c.domain.clone()
            } else {
                format!(".{}", c.domain)
            };
            let flag = if c.domain.starts_with('.') { "TRUE" } else { "FALSE" };
            let secure = if c.secure { "TRUE" } else { "FALSE" };
            let expires = c.expires.map(|e| e as u64).unwrap_or(0);
            lines.push(format!(
                "{}\t{}\t{}\t{}\t{}\t{}",
                domain, flag, c.path, secure, expires, c.name
            ));
            // Netscape format: name is last, but value needs special handling
            // Actually format is: domain\tflag\tpath\tsecure\texpires\tname\tvalue
            lines.pop(); // remove the wrong line
            lines.push(format!(
                "{}\t{}\t{}\t{}\t{}\t{}\t{}",
                domain, flag, c.path, secure, expires, c.name, c.value
            ));
        }
        lines.join("\n")
    }

    /// Import from Netscape cookies.txt format
    pub fn from_netscape(text: &str) -> Self {
        let mut jar = Self::new();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() < 7 {
                continue;
            }
            let domain = parts[0].to_string();
            let path = parts[2].to_string();
            let secure = parts[3] == "TRUE";
            let expires: Option<f64> = parts[4].parse().ok().filter(|&v| v > 0.0);
            let name = parts[5].to_string();
            let value = parts[6].to_string();

            jar.add(Cookie {
                name,
                value,
                domain,
                path,
                expires,
                http_only: false,
                secure,
                same_site: None,
            });
        }
        jar
    }

    /// Export to HTTP header format (for use with --cookie flag)
    pub fn to_header_format(&self) -> Vec<String> {
        self.cookies.iter().map(|c| format!("{}={}", c.name, c.value)).collect()
    }

    /// Import from HTTP Cookie header string
    pub fn from_cookie_header(header: &str) -> Self {
        let mut jar = Self::new();
        for part in header.split(';') {
            let part = part.trim();
            if let Some((name, value)) = part.split_once('=') {
                jar.add(Cookie::new(name.trim(), value.trim()));
            }
        }
        jar
    }
}

/// Cookie manager with browser connection
pub struct CookieManager {
    connection: Arc<CdpConnection>,
}

impl CookieManager {
    pub fn new(connection: Arc<CdpConnection>) -> Self {
        Self { connection }
    }

    /// Get all cookies for given URLs
    pub async fn get_cookies(&self, urls: &[&str]) -> Result<Vec<Cookie>> {
        let url_values: Vec<serde_json::Value> =
            urls.iter().map(|u| serde_json::Value::String(u.to_string())).collect();
        let result = self
            .connection
            .send_page(
                "Network.getCookies",
                serde_json::json!({ "urls": url_values }),
            )
            .await?;

        let cookies = result
            .get("cookies")
            .and_then(|c| c.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|c| {
                        Some(Cookie {
                            name: c["name"].as_str()?.to_string(),
                            value: c["value"].as_str()?.to_string(),
                            domain: c["domain"].as_str().unwrap_or("").to_string(),
                            path: c["path"].as_str().unwrap_or("").to_string(),
                            expires: c["expires"].as_f64(),
                            http_only: c["httpOnly"].as_bool().unwrap_or(false),
                            secure: c["secure"].as_bool().unwrap_or(false),
                            same_site: c["sameSite"].as_str().map(|s| s.to_string()),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(cookies)
    }

    /// Get all cookies as a CookieJar
    pub async fn get_jar(&self, urls: &[&str]) -> Result<CookieJar> {
        let cookies = self.get_cookies(urls).await?;
        let mut jar = CookieJar::new();
        jar.add_many(cookies);
        Ok(jar)
    }

    /// Set a cookie
    pub async fn set_cookie(&self, cookie: &Cookie) -> Result<()> {
        let mut params = serde_json::json!({
            "name": cookie.name,
            "value": cookie.value,
            "domain": cookie.domain,
            "path": cookie.path,
            "httpOnly": cookie.http_only,
            "secure": cookie.secure,
        });
        if let Some(expires) = cookie.expires {
            params["expires"] = serde_json::json!(expires);
        }
        if let Some(ref same_site) = cookie.same_site {
            params["sameSite"] = serde_json::Value::String(same_site.clone());
        }

        self.connection
            .send_page("Network.setCookie", params)
            .await?;
        Ok(())
    }

    /// Set multiple cookies from a jar
    pub async fn set_jar(&self, jar: &CookieJar) -> Result<()> {
        for cookie in jar.all() {
            self.set_cookie(cookie).await?;
        }
        Ok(())
    }

    /// Delete a specific cookie
    pub async fn delete_cookie(&self, name: &str, domain: &str, path: &str) -> Result<()> {
        self.connection
            .send_page(
                "Network.deleteCookies",
                serde_json::json!({
                    "name": name,
                    "domain": domain,
                    "path": path,
                }),
            )
            .await?;
        Ok(())
    }

    /// Delete cookies by name (all domains)
    pub async fn delete_by_name(&self, name: &str) -> Result<()> {
        let cookies = self.get_cookies(&["http://"]).await?;
        for c in cookies.iter().filter(|c| c.name == name) {
            self.delete_cookie(&c.name, &c.domain, &c.path).await?;
        }
        Ok(())
    }

    /// Clear all cookies
    pub async fn clear_all(&self) -> Result<()> {
        self.connection
            .send_page("Network.clearBrowserCookies", serde_json::json!({}))
            .await?;
        Ok(())
    }

    /// Get cookies as a HashMap for easy lookup
    pub async fn get_cookie_map(&self, urls: &[&str]) -> Result<HashMap<String, String>> {
        let cookies = self.get_cookies(urls).await?;
        let map: HashMap<String, String> = cookies
            .into_iter()
            .map(|c| (c.name, c.value))
            .collect();
        Ok(map)
    }

    /// Check if a cookie exists
    pub async fn has_cookie(&self, urls: &[&str], name: &str) -> Result<bool> {
        let cookies = self.get_cookies(urls).await?;
        Ok(cookies.iter().any(|c| c.name == name))
    }

    /// Export browser cookies to file
    pub async fn export_json(&self, urls: &[&str], path: &str) -> Result<()> {
        let jar = self.get_jar(urls).await?;
        jar.save_json(path).await
    }

    /// Export browser cookies to Netscape format
    pub async fn export_netscape(&self, urls: &[&str], path: &str) -> Result<()> {
        let jar = self.get_jar(urls).await?;
        let content = jar.to_netscape();
        tokio::fs::write(path, content).await?;
        Ok(())
    }

    /// Import cookies from file into browser
    pub async fn import_json(&self, path: &str) -> Result<usize> {
        let jar = CookieJar::load_json(path).await?;
        let count = jar.len();
        self.set_jar(&jar).await?;
        Ok(count)
    }

    /// Import cookies from Netscape format
    pub async fn import_netscape(&self, path: &str) -> Result<usize> {
        let content = tokio::fs::read_to_string(path).await?;
        let jar = CookieJar::from_netscape(&content);
        let count = jar.len();
        self.set_jar(&jar).await?;
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cookie_new() {
        let c = Cookie::new("sid", "abc");
        assert_eq!(c.name, "sid");
        assert_eq!(c.value, "abc");
        assert_eq!(c.path, "/");
    }

    #[test]
    fn test_cookie_builder() {
        let c = Cookie::new("token", "xyz")
            .domain(".example.com")
            .path("/api")
            .expires(1234567890.0)
            .http_only(true)
            .secure(true)
            .same_site("Strict");
        assert_eq!(c.domain, ".example.com");
        assert!(c.http_only);
        assert!(c.secure);
    }

    #[test]
    fn test_cookie_is_expired() {
        let c = Cookie { expires: Some(0.0), ..Cookie::new("a", "b") };
        assert!(c.is_expired());
        let c = Cookie { expires: Some(9999999999.0), ..Cookie::new("a", "b") };
        assert!(!c.is_expired());
        let c = Cookie { expires: None, ..Cookie::new("a", "b") };
        assert!(!c.is_expired());
    }

    #[test]
    fn test_cookie_domain_matches() {
        let c = Cookie { domain: ".example.com".to_string(), ..Cookie::new("a", "b") };
        assert!(c.domain_matches("www.example.com"));
        assert!(c.domain_matches("example.com"));
        assert!(!c.domain_matches("other.com"));

        let c = Cookie { domain: "example.com".to_string(), ..Cookie::new("a", "b") };
        assert!(c.domain_matches("example.com"));
        assert!(!c.domain_matches("www.example.com"));
    }

    #[test]
    fn test_cookie_to_set_cookie_header() {
        let c = Cookie::new("sid", "abc")
            .domain(".example.com")
            .http_only(true)
            .secure(true);
        let header = c.to_set_cookie_header();
        assert!(header.contains("sid=abc"));
        assert!(header.contains("Domain=.example.com"));
        assert!(header.contains("HttpOnly"));
        assert!(header.contains("Secure"));
    }

    #[test]
    fn test_cookie_to_cookie_header() {
        let c = Cookie::new("session", "123");
        assert_eq!(c.to_cookie_header(), "session=123");
    }

    #[test]
    fn test_jar_add_and_get() {
        let mut jar = CookieJar::new();
        jar.add(Cookie::new("a", "1"));
        jar.add(Cookie::new("b", "2"));
        assert_eq!(jar.len(), 2);
        assert_eq!(jar.get_value("a"), Some("1"));
    }

    #[test]
    fn test_jar_add_replaces_same_key() {
        let mut jar = CookieJar::new();
        jar.add(Cookie::new("a", "1").domain(".test.com"));
        jar.add(Cookie::new("a", "2").domain(".test.com"));
        assert_eq!(jar.len(), 1);
        assert_eq!(jar.get_value("a"), Some("2"));
    }

    #[test]
    fn test_jar_remove() {
        let mut jar = CookieJar::new();
        jar.add(Cookie::new("a", "1").domain(".test.com"));
        assert!(jar.remove("a", ".test.com", "/"));
        assert!(jar.is_empty());
    }

    #[test]
    fn test_jar_remove_by_name() {
        let mut jar = CookieJar::new();
        jar.add(Cookie::new("a", "1"));
        jar.add(Cookie::new("a", "2").domain(".test.com"));
        jar.add(Cookie::new("b", "3"));
        let removed = jar.remove_by_name("a");
        assert_eq!(removed, 2);
        assert_eq!(jar.len(), 1);
    }

    #[test]
    fn test_jar_remove_by_domain() {
        let mut jar = CookieJar::new();
        jar.add(Cookie::new("a", "1").domain(".test.com"));
        jar.add(Cookie::new("b", "2").domain("other.com"));
        let removed = jar.remove_by_domain("test.com");
        assert_eq!(removed, 1);
    }

    #[test]
    fn test_jar_remove_expired() {
        let mut jar = CookieJar::new();
        jar.add(Cookie { expires: Some(0.0), ..Cookie::new("expired", "1") });
        jar.add(Cookie::new("valid", "2"));
        let removed = jar.remove_expired();
        assert_eq!(removed, 1);
        assert_eq!(jar.len(), 1);
    }

    #[test]
    fn test_jar_to_cookie_header() {
        let mut jar = CookieJar::new();
        jar.add(Cookie::new("a", "1"));
        jar.add(Cookie::new("b", "2"));
        let header = jar.to_cookie_header();
        assert!(header.contains("a=1"));
        assert!(header.contains("b=2"));
    }

    #[test]
    fn test_jar_from_cookie_header() {
        let jar = CookieJar::from_cookie_header("a=1; b=2; c=3");
        assert_eq!(jar.len(), 3);
        assert_eq!(jar.get_value("b"), Some("2"));
    }

    #[test]
    fn test_jar_merge() {
        let mut jar1 = CookieJar::new();
        jar1.add(Cookie::new("a", "1"));
        let mut jar2 = CookieJar::new();
        jar2.add(Cookie::new("b", "2"));
        jar2.add(Cookie::new("a", "3"));
        jar1.merge(&jar2);
        assert_eq!(jar1.len(), 2);
        assert_eq!(jar1.get_value("a"), Some("3"));
    }

    #[test]
    fn test_jar_filter() {
        let mut jar = CookieJar::new();
        jar.add(Cookie::new("sid", "1").domain(".example.com"));
        jar.add(Cookie::new("token", "2").domain("other.com"));
        let filtered = jar.filter(|c| c.domain.contains("example"));
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn test_jar_to_map() {
        let mut jar = CookieJar::new();
        jar.add(Cookie::new("a", "1"));
        jar.add(Cookie::new("b", "2"));
        let map = jar.to_map();
        assert_eq!(map.len(), 2);
        assert_eq!(map["a"], "1");
    }

    #[test]
    fn test_jar_to_netscape() {
        let mut jar = CookieJar::new();
        jar.add(Cookie::new("sid", "abc")
            .domain(".example.com")
            .path("/")
            .secure(true)
            .expires(1234567890.0));
        let netscape = jar.to_netscape();
        assert!(netscape.contains("Netscape HTTP Cookie File"));
        assert!(netscape.contains("example.com"));
        assert!(netscape.contains("sid"));
        assert!(netscape.contains("abc"));
    }

    #[test]
    fn test_jar_from_netscape() {
        let txt = "# Netscape HTTP Cookie File\n\
            .example.com\tTRUE\t/\tTRUE\t1234567890\tsid\tabc\n\
            .other.com\tFALSE\t/\tFALSE\t0\ttoken\txyz\n";
        let jar = CookieJar::from_netscape(txt);
        assert_eq!(jar.len(), 2);
        assert_eq!(jar.get_value("sid"), Some("abc"));
    }

    #[test]
    fn test_jar_json_roundtrip() {
        let mut jar = CookieJar::new();
        jar.add(Cookie::new("a", "1").domain(".test.com"));
        jar.add(Cookie::new("b", "2"));
        let json = serde_json::to_string(&jar.cookies).unwrap();
        let cookies: Vec<Cookie> = serde_json::from_str(&json).unwrap();
        let jar2 = CookieJar { cookies };
        assert_eq!(jar2.len(), 2);
    }

    #[test]
    fn test_cookie_serialize() {
        let c = Cookie::new("test", "123");
        let json = serde_json::to_string(&c).unwrap();
        assert!(json.contains("test"));
    }

    #[test]
    fn test_cookie_deserialize() {
        let json = r#"{"name":"sid","value":"xyz","domain":".example.com","path":"/","expires":null,"httpOnly":true,"secure":true,"sameSite":"Strict"}"#;
        let c: Cookie = serde_json::from_str(json).unwrap();
        assert_eq!(c.name, "sid");
        assert!(c.http_only);
    }
}
