// find_endorsers_rust.rs - Find endorsers using arXiv API directly
use std::io::{Read, Write};
use native_tls::TlsStream;
use std::net::TcpStream;

const CATEGORIES: &[(&str, &str)] = &[
    ("math.LO", "NWTCV4"),
    ("math.GM", "WUYN9M"),
    ("math.CO", "HBLFEF"),
    ("math.NT", "SQLB7M"),
    ("math.PR", "B3QW4D"),
    ("cs.AI", "TDF9EK"),
    ("cs.CR", "QLKH39"),
    ("cs.LO", "K8ZWC9"),
];

fn fetch_url(url: &str) -> Result<String, String> {
    let mut stream = TcpStream::connect("export.arxiv.org:80")
        .map_err(|e| format!("Connection failed: {}", e))?;
    
    let request = format!(
        "GET {} HTTP/1.1\r\nHost: export.arxiv.org\r\nConnection: close\r\n\r\n",
        url
    );
    
    stream.write_all(request.as_bytes()).map_err(|e| e.to_string())?;
    
    let mut response = String::new();
    stream.read_to_string(&mut response).map_err(|e| e.to_string())?;
    
    // Extract body (after \r\n\r\n)
    if let Some(pos) = response.find("\r\n\r\n") {
        Ok(response[pos + 4..].to_string())
    } else {
        Ok(response)
    }
}

fn extract_authors_from_xml(xml: &str) -> Vec<(String, String)> {
    let mut authors = Vec::new();
    let mut i = 0;
    let bytes = xml.as_bytes();
    
    while i < bytes.len() {
        // Look for <author> tag
        if xml[i..].starts_with("<author>") {
            let start = i + 8;
            if let Some(end) = xml[start..].find("</author>") {
                let author_block = &xml[start..start + end];
                
                // Extract name
                if let Some(name_start) = author_block.find("<name>") {
                    let name_begin = name_start + 6;
                    if let Some(name_end) = author_block[name_begin..].find("</name>") {
                        let name = author_block[name_begin..name_begin + name_end].trim().to_string();
                        
                        // Extract email
                        let email = if let Some(email_start) = author_block.find("<email>") {
                            let email_begin = email_start + 7;
                            if let Some(email_end) = author_block[email_begin..].find("</email>") {
                                author_block[email_begin..email_begin + email_end].trim().to_string()
                            } else {
                                String::new()
                            }
                        } else {
                            String::new()
                        };
                        
                        if !name.is_empty() {
                            authors.push((name, email));
                        }
                    }
                }
                
                i = start + end + 9;
            } else {
                i += 8;
            }
        } else {
            i += 1;
        }
    }
    
    authors
}

struct SmtpSender {
    stream: TcpStream,
    tls_stream: Option<TlsStream<TcpStream>>,
}

impl SmtpSender {
    fn new() -> Result<Self, String> {
        let stream = TcpStream::connect("smtp.gmail.com:587")
            .map_err(|e| format!("SMTP connection failed: {}", e))?;
        
        let mut sender = SmtpSender { stream, tls_stream: None };
        let mut buf = [0u8; 4096];
        let n = sender.stream.read(&mut buf).unwrap_or(0);
        println!("[SMTP] {}", String::from_utf8_lossy(&buf[..n]).trim());
        
        Ok(sender)
    }
    
    fn send_command(&mut self, command: &str) -> Result<String, String> {
        let cmd = format!("{}\r\n", command);
        
        if let Some(ref mut tls) = self.tls_stream {
            tls.write_all(cmd.as_bytes()).map_err(|e| e.to_string())?;
            let mut buf = [0u8; 4096];
            let n = tls.read(&mut buf).map_err(|e| e.to_string())?;
            Ok(String::from_utf8_lossy(&buf[..n]).to_string())
        } else {
            self.stream.write_all(cmd.as_bytes()).map_err(|e| e.to_string())?;
            let mut buf = [0u8; 4096];
            let n = self.stream.read(&mut buf).map_err(|e| e.to_string())?;
            Ok(String::from_utf8_lossy(&buf[..n]).to_string())
        }
    }
    
    fn starttls(&mut self) -> Result<(), String> {
        let r = self.send_command("STARTTLS")?;
        if !r.contains("220") { return Err(format!("STARTTLS failed: {}", r)); }
        
        let connector = native_tls::TlsConnector::new().map_err(|e| e.to_string())?;
        let tls = connector.connect("smtp.gmail.com", self.stream.try_clone().map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        self.tls_stream = Some(tls);
        Ok(())
    }
    
    fn auth(&mut self, user: &str, pass: &str) -> Result<(), String> {
        use base64::Engine;
        
        let _ = self.send_command("EHLO localhost");
        self.starttls()?;
        let _ = self.send_command("EHLO localhost");
        
        let r = self.send_command("AUTH LOGIN")?;
        if !r.contains("334") { return Err(format!("AUTH LOGIN failed: {}", r)); }
        
        let user_b64 = base64::engine::general_purpose::STANDARD.encode(user);
        let _ = self.send_command(&user_b64)?;
        
        let pass_b64 = base64::engine::general_purpose::STANDARD.encode(pass);
        let r = self.send_command(&pass_b64)?;
        
        if r.contains("235") { Ok(()) } else { Err(format!("Auth failed: {}", r)) }
    }
    
    fn send_email(&mut self, from: &str, to: &str, subject: &str, body: &str) -> Result<(), String> {
        use base64::Engine;
        
        let _ = self.send_command(&format!("MAIL FROM:<{}>", from));
        let _ = self.send_command(&format!("RCPT TO:<{}>", to));
        let r = self.send_command("DATA")?;
        if !r.contains("354") { return Err(format!("DATA failed: {}", r)); }
        
        let subject_b64 = base64::engine::general_purpose::STANDARD.encode(subject.as_bytes());
        let email = format!(
            "From: <{}>\r\n\
             To: <{}>\r\n\
             Subject: =?UTF-8?B?{}?=\r\n\
             MIME-Version: 1.0\r\n\
             Content-Type: text/plain; charset=UTF-8\r\n\
             \r\n\
             {}\r\n\
             .\r\n",
            from, to, subject_b64, body
        );
        
        let r = self.send_command(&email)?;
        if r.contains("250") { Ok(()) } else { Err(format!("Send failed: {}", r)) }
    }
}

fn generate_email(name: &str, category: &str, code: &str) -> String {
    format!(
        "Dear {},\n\n\
         I am writing to request your endorsement for my submission to arXiv.\n\n\
         Paper: \"1001 Proofs: A Rigorous Collection with Explicit Assumptions, \n\
         Dependencies, and Verification Boundaries\"\n\n\
         To endorse for {}, please:\n\
         1. Visit: https://arxiv.org/auth/endorse.php\n\
         2. Enter code: {}\n\n\
         Best regards,\n\
         Yuriy Aronov\n\
         apohob5@gmail.com",
        name, category, code
    )
}

fn main() {
    println!("========================================");
    println!("  Finding arXiv Endorsers (Rust)");
    println!("========================================\n");
    
    let mut all_endorsers: Vec<(String, String, String, String)> = Vec::new(); // (name, email, category, code)
    
    // Step 1: Search arXiv for papers in each category
    println!("[1] Searching arXiv for active researchers...\n");
    
    for (cat, code) in CATEGORIES {
        println!("Searching {}...", cat);
        
        let url = format!(
            "/api/query?search_query=cat:{}&max_results=30&sortBy=submittedDate&sortOrder=descending",
            cat
        );
        
        match fetch_url(&url) {
            Ok(xml) => {
                let authors = extract_authors_from_xml(&xml);
                println!("  Found {} authors", authors.len());
                
                for (name, email) in authors {
                    all_endorsers.push((name, email, cat.to_string(), code.to_string()));
                }
            }
            Err(e) => {
                println!("  Error: {}", e);
            }
        }
        
        // Rate limit: wait 1 second between requests
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
    
    // Remove duplicates by name
    all_endorsers.sort_by(|a, b| a.0.cmp(&b.0));
    all_endorsers.dedup_by(|a, b| a.0 == b.0);
    
    println!("\n[2] Total unique endorsers: {}", all_endorsers.len());
    
    // Step 2: Save endorsers list
    println!("\n[3] Saving endorsers list...");
    
    let mut file = std::fs::File::create("E:\\1\\endorsers_rust.txt").unwrap();
    writeln!(file, "# arXiv Endorsers List ({} total)\n", all_endorsers.len()).unwrap();
    writeln!(file, "Category | Name | Email | Code").unwrap();
    writeln!(file, "---------|------|-------|-----").unwrap();
    
    for (name, email, cat, code) in &all_endorsers {
        writeln!(file, "{} | {} | {} | {}", cat, name, email, code).unwrap();
    }
    
    println!("  Saved to: E:\\1\\endorsers_rust.txt");
    
    // Step 3: Print summary
    println!("\n========================================");
    println!("  Endorsers by Category");
    println!("========================================");
    
    for (cat, code) in CATEGORIES {
        let count = all_endorsers.iter().filter(|e| e.2 == *cat).count();
        println!("  {}: {} endorsers (code: {})", cat, count, code);
    }
    
    println!("\n========================================");
    println!("  Done! File: E:\\1\\endorsers_rust.txt");
    println!("========================================");
}
