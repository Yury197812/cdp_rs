// email/validator/smtp_check.rs - SMTP-based email verification
use std::io::{Read, Write};
use std::net::TcpStream;

pub fn smtp_verify(address: &str, smtp_host: &str) -> bool {
    let stream = match TcpStream::connect(format!("{}:25", smtp_host)) {
        Ok(s) => s,
        Err(_) => return false,
    };
    
    let mut stream = stream;
    let mut buffer = [0u8; 1024];
    
    // Read greeting
    let _ = stream.read(&mut buffer);
    
    // HELO
    let _ = stream.write_all(b"HELO test\r\n");
    let _ = stream.read(&mut buffer);
    
    // MAIL FROM
    let _ = stream.write_all(b"MAIL FROM:<test@test.com>\r\n");
    let _ = stream.read(&mut buffer);
    
    // RCPT TO
    let _ = stream.write_all(format!("RCPT TO:<{}>\r\n", address).as_bytes());
    let _ = stream.read(&mut buffer);
    
    let response = String::from_utf8_lossy(&buffer);
    
    // 250 = OK, 452 = sufficient storage (also OK)
    response.contains("250") || response.contains("452")
}
