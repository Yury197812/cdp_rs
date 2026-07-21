// find_400_endorsers.rs - Find 400 active endorsers across all categories
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
    
    fn logout(&mut self) { let _ = self.send_command("LOGOUT"); }
}

// Categories and codes
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

// SMTP sender
struct SmtpSender {
    stream: TcpStream,
    tls_stream: Option<TlsStream<TcpStream>>,
}

impl SmtpSender {
    fn new() -> Result<Self, String> {
        let stream = TcpStream::connect("smtp.gmail.com:587")
            .map_err(|e| format!("SMTP connection failed: {}", e))?;
        
        let mut sender = SmtpSender {
            stream,
            tls_stream: None,
        };
        
        // Read greeting
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
        let r = self.send_command(&user_b64)?;
        if !r.contains("334") { return Err(format!("Username failed: {}", r)); }
        
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
    println!("  Finding 400 Active Endorsers");
    println!("========================================\n");
    
    // Step 1: Search arXiv for active researchers via web
    println!("[1] Searching arXiv for active researchers...\n");
    
    // We'll use curl to search arXiv API
    let mut endorsers: Vec<(String, String, String)> = Vec::new(); // (name, email, category)
    
    for (cat, code) in CATEGORIES {
        println!("Searching {}...", cat);
        
        // Use curl to search arXiv
        let search_url = format!(
            "http://export.arxiv.org/api/query?search_query=cat:{}&max_results=50&sortBy=submittedDate&sortOrder=descending",
            cat
        );
        
        let output = std::process::Command::new("curl")
            .args(&["-s", &search_url])
            .output();
        
        if let Ok(output) = output {
            let xml = String::from_utf8_lossy(&output.stdout).to_string();
            
            // Extract author emails from XML
            let lines: Vec<&str> = xml.split('\n').collect();
            let mut i = 0;
            while i < lines.len() {
                if lines[i].contains("<author>") {
                    // Extract name
                    let name = if let Some(next) = lines.get(i + 1) {
                        next.trim().replace("<name>", "").replace("</name>", "").trim().to_string()
                    } else {
                        String::new()
                    };
                    
                    // Extract email
                    let email = if let Some(next) = lines.get(i + 2) {
                        let e = next.trim().replace("<email>", "").replace("</email>", "").trim().to_string();
                        if e.contains('@') { e } else { String::new() }
                    } else {
                        String::new()
                    };
                    
                    if !email.is_empty() && email.contains('@') && !email.ends_with(".invalid") {
                        endorsers.push((name.clone(), email.clone(), cat.to_string()));
                    }
                    
                    i += 5;
                } else {
                    i += 1;
                }
            }
        }
        
        println!("  Found {} researchers", endorsers.iter().filter(|e| e.2 == *cat).count());
    }
    
    println!("\n[2] Total endorsers found: {}", endorsers.len());
    
    // Step 2: Save endorsers list
    println!("\n[3] Saving endorsers list...");
    
    let mut file = std::fs::File::create("E:\\1\\endorsers_400.txt").unwrap();
    writeln!(file, "# arXiv Endorsers List ({} total)\n", endorsers.len()).unwrap();
    writeln!(file, "Category | Name | Email | Code").unwrap();
    writeln!(file, "---------|------|-------|-----").unwrap();
    
    for (name, email, cat) in &endorsers {
        let code = CATEGORIES.iter().find(|(c, _)| c == cat).map(|(_, c)| *c).unwrap_or("UNKNOWN");
        writeln!(file, "{} | {} | {} | {}", cat, name, email, code).unwrap();
    }
    
    println!("  Saved to: E:\\1\\endorsers_400.txt");
    
    // Step 3: Send emails (first 50 to test)
    println!("\n[4] Sending emails (first 50)...");
    
    let mut smtp = match SmtpSender::new() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("SMTP connection failed: {}", e);
            return;
        }
    };
    
    if let Err(e) = smtp.auth("apohob5@gmail.com", "zkpsgveafmrnldrt") {
        eprintln!("SMTP auth failed: {}", e);
        return;
    }
    
    let mut sent = 0;
    let mut failed = 0;
    
    for (name, email, cat) in endorsers.iter().take(50) {
        let code = CATEGORIES.iter().find(|(c, _)| *c == cat.as_str()).map(|(_, c)| *c).unwrap_or("UNKNOWN");
        let subject = format!("Endorsement request for arXiv - {}", cat);
        let body = generate_email(name, cat, code);
        
        match smtp.send_email("apohob5@gmail.com", email, &subject, &body) {
            Ok(_) => {
                println!("  ✓ {} <{}>", name, email);
                sent += 1;
            }
            Err(e) => {
                println!("  ✗ {} <{}>: {}", name, email, e);
                failed += 1;
            }
        }
        
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
    
    println!("\n========================================");
    println!("  Summary");
    println!("========================================");
    println!("  Endorsers found: {}", endorsers.len());
    println!("  Emails sent: {}", sent);
    println!("  Failed: {}", failed);
    println!("  List saved: E:\\1\\endorsers_400.txt");
    println!("========================================");
}
