// check_responses.rs - Check for endorser responses
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
    println!("  Checking for Endorser Responses");
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
    
    // Search for recent emails (today)
    println!("Searching for recent emails...");
    let ids = client.search("SINCE \"20-Jul-2026\"").unwrap_or_default();
    println!("Found {} recent emails\n", ids.len());
    
    let mut responses = Vec::new();
    
    for id in &ids {
        if let Ok(body) = client.fetch(*id, "RFC822") {
            let body_lower = body.to_lowercase();
            
            // Check if it's an endorser response (not our sent emails)
            if body_lower.contains("endorse") && 
               !body_lower.contains("endorsement request from yuriy") &&
               !body_lower.contains("from: yuriy") {
                
                // Extract subject and sender
                let mut subject = String::new();
                let mut from = String::new();
                
                for line in body.lines() {
                    let line_lower = line.to_lowercase();
                    if line_lower.starts_with("subject:") {
                        subject = line[8..].trim().to_string();
                    }
                    if line_lower.starts_with("from:") {
                        from = line[5..].trim().to_string();
                    }
                }
                
                if !subject.is_empty() {
                    responses.push((id.clone(), from, subject));
                }
            }
        }
    }
    
    client.logout();
    
    if responses.is_empty() {
        println!("No endorser responses found yet.");
        println!("\nThe emails we sent are still pending.");
        println!("Expected response time: 1-7 days");
    } else {
        println!("=== ENDORSER RESPONSES ===\n");
        for (id, from, subject) in &responses {
            println!("Email #{}:", id);
            println!("  From: {}", from);
            println!("  Subject: {}", subject);
            println!();
        }
    }
    
    println!("========================================");
    println!("  Next check: Run again in 24 hours");
    println!("========================================");
}
