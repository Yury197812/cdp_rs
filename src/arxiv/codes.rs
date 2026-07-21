// find_codes.rs - Extract all endorsement codes from arXiv emails
use native_tls::TlsStream;
use std::io::{Read, Write};
use std::net::TcpStream;

struct ImapClient {
    stream: TlsStream<TcpStream>,
    tag: u32,
}

impl ImapClient {
    fn new(server: &str, port: u16) -> Result<Self, String> {
        let connector = native_tls::TlsConnector::new().map_err(|e| e.to_string())?;
        let tcp = TcpStream::connect(format!("{}:{}", server, port)).map_err(|e| e.to_string())?;
        let stream = connector.connect(server, tcp).map_err(|e| e.to_string())?;
        let mut client = ImapClient { stream, tag: 0 };
        let _ = client.read_response();
        Ok(client)
    }
    
    fn send_command(&mut self, command: &str) -> Result<String, String> {
        self.tag += 1;
        let tag_str = format!("A{:04}", self.tag);
        self.stream.write_all(format!("{} {}\r\n", tag_str, command).as_bytes()).map_err(|e| e.to_string())?;
        self.read_response()
    }
    
    fn read_response(&mut self) -> Result<String, String> {
        let mut response = String::new();
        let mut buffer = [0u8; 16384];
        loop {
            match self.stream.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => {
                    response.push_str(&String::from_utf8_lossy(&buffer[..n]));
                    if response.ends_with("\r\n") { break; }
                }
                Err(_) => break,
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        Ok(response)
    }
    
    fn login(&mut self, user: &str, password: &str) -> Result<(), String> {
        let r = self.send_command(&format!("LOGIN {} {}", user, password))?;
        if r.contains("OK") { Ok(()) } else { Err(r) }
    }
    
    fn select(&mut self, folder: &str) -> Result<u32, String> {
        let r = self.send_command(&format!("SELECT \"{}\"", folder))?;
        for line in r.lines() {
            if line.contains("EXISTS") {
                if let Some(n) = line.split_whitespace().find_map(|s| s.parse().ok()) {
                    return Ok(n);
                }
            }
        }
        Ok(0)
    }
    
    fn search(&mut self, query: &str) -> Result<Vec<u32>, String> {
        let r = self.send_command(&format!("SEARCH {}", query))?;
        let mut ids = Vec::new();
        for line in r.lines() {
            if line.starts_with("* SEARCH") {
                if let Some(ids_str) = line.split_once("* SEARCH ") {
                    for id in ids_str.1.split_whitespace() {
                        if let Ok(n) = id.parse::<u32>() { ids.push(n); }
                    }
                }
            }
        }
        Ok(ids)
    }
    
    fn fetch(&mut self, id: u32, section: &str) -> Result<String, String> {
        self.send_command(&format!("FETCH {} {}", id, section))
    }
    
    fn logout(&mut self) { let _ = self.send_command("LOGOUT"); }
}

fn main() {
    println!("========================================");
    println!("  Extracting arXiv Endorsement Codes");
    println!("========================================\n");
    
    let mut client = match ImapClient::new("imap.gmail.com", 993) {
        Ok(c) => c,
        Err(e) => { eprintln!("Connection failed: {}", e); return; }
    };
    
    if let Err(e) = client.login("apohob5@gmail.com", "zkpsgveafmrnldrt") {
        eprintln!("Login failed: {}", e);
        return;
    }
    
    client.select("INBOX").unwrap_or(0);
    
    // Search for all emails from arxiv
    let ids = client.search("FROM \"arxiv.org\"").unwrap_or_default();
    println!("Found {} arXiv emails\n", ids.len());
    
    let mut codes: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    
    for id in &ids {
        if let Ok(body) = client.fetch(*id, "RFC822") {
            if body.to_lowercase().contains("endorsement code") {
                // Extract endorsement code
                let mut code = String::new();
                let mut category = String::new();
                
                for line in body.lines() {
                    let lower = line.to_lowercase();
                    if lower.contains("endorsement code") && line.contains(":") {
                        if let Some(c) = line.split(':').last() {
                            code = c.trim().to_string();
                        }
                    }
                    if lower.contains("section of arxiv") || lower.contains("subject class") {
                        // Extract category like "cs.AI" or "math.LO"
                        let words: Vec<&str> = line.split_whitespace().collect();
                        for w in &words {
                            if w.contains('.') && (w.starts_with("math.") || w.starts_with("cs.") || w.starts_with("stat.")) {
                                category = w.trim_matches(|c: char| !c.is_alphanumeric() && c != '.').to_string();
                            }
                        }
                    }
                }
                
                if !code.is_empty() && code.len() == 6 {
                    if !category.is_empty() {
                        codes.insert(category.clone(), code.clone());
                        println!("Category: {} -> Code: {}", category, code);
                    } else {
                        // Try to determine category from context
                        println!("Code found: {} (category unknown, check email)", code);
                    }
                }
            }
        }
    }
    
    client.logout();
    
    // Summary
    println!("\n========================================");
    println!("  ENDORSEMENT CODES SUMMARY");
    println!("========================================");
    println!("\nCopy these codes to endorsers:\n");
    
    let all_categories = vec![
        "math.LO", "math.GM", "math.CO", "math.NT", "math.PR",
        "cs.AI", "cs.CR", "cs.LO", "cs.DS"
    ];
    
    for cat in &all_categories {
        if let Some(code) = codes.get(*cat) {
            println!("  {}: {}", cat, code);
        } else {
            println!("  {}: CHECK GMAIL (not found in recent emails)", cat);
        }
    }
    
    println!("\n========================================");
    println!("  How to use:");
    println!("  1. Forward this info to endorsers");
    println!("  2. Endorser goes to: https://arxiv.org/auth/endorse.php");
    println!("  3. Enters the code for their category");
    println!("========================================");
}
