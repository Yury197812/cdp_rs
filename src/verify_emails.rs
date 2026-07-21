// verify_emails.rs - Verify email addresses via SMTP RCPT TO
use std::io::{Read, Write};
use std::net::TcpStream;

fn verify_email(mx_server: &str, from: &str, to: &str) -> bool {
    // Connect to MX server
    let mut stream = match TcpStream::connect(format!("{}:25", mx_server)) {
        Ok(s) => s,
        Err(_) => return false,
    };
    
    // Read greeting
    let mut buf = [0u8; 1024];
    let _ = stream.read(&mut buf);
    
    // Send EHLO
    let _ = stream.write_all(b"EHLO verify.test\r\n");
    let _ = stream.read(&mut buf);
    
    // Send MAIL FROM
    let _ = stream.write_all(format!("MAIL FROM:<{}>\r\n", from).as_bytes());
    let _ = stream.read(&mut buf);
    
    // Send RCPT TO
    let _ = stream.write_all(format!("RCPT TO:<{}>\r\n", to).as_bytes());
    let mut response = [0u8; 1024];
    let n = stream.read(&mut response).unwrap_or(0);
    let response_str = String::from_utf8_lossy(&response[..n]);
    
    // Send QUIT
    let _ = stream.write_all(b"QUIT\r\n");
    
    // Check if 250 OK (email exists)
    response_str.contains("250") && !response_str.contains("550") && !response_str.contains("551")
}

fn get_mx_server(domain: &str) -> Option<String> {
    // Simple MX lookup using DNS
    let output = std::process::Command::new("nslookup")
        .args(&["-type=mx", domain])
        .output()
        .ok()?;
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Parse MX record
    for line in stdout.lines() {
        if line.contains("mail exchanger") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                return Some(parts[3].trim_end_matches('.').to_string());
            }
        }
    }
    None
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    
    if args.len() < 2 {
        println!("Usage: verify_emails <email1> <email2> ...");
        println!("Example: verify_emails user@gmail.com test@yahoo.com");
        return;
    }
    
    println!("========================================");
    println!("  Email Verification (SMTP RCPT TO)");
    println!("========================================\n");
    
    for email in &args[1..] {
        let parts: Vec<&str> = email.split('@').collect();
        if parts.len() != 2 {
            println!("  ✗ {} - Invalid format", email);
            continue;
        }
        
        let domain = parts[1];
        print!("  Checking {}...", email);
        
        // Get MX server
        let mx = match get_mx_server(domain) {
            Some(mx) => mx,
            None => {
                println!(" ✗ No MX record for {}", domain);
                continue;
            }
        };
        
        // Verify via SMTP
        if verify_email(&mx, "verify@test.com", email) {
            println!(" ✓ VALID");
        } else {
            println!(" ✗ INVALID");
        }
    }
    
    println!("\n========================================");
}
