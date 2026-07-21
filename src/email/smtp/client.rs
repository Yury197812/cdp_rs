// email/smtp/client.rs - SMTP client implementation
use native_tls::TlsStream;
use std::io::{Read, Write};
use std::net::TcpStream;

pub struct SmtpClient {
    stream: TcpStream,
    tls_stream: Option<TlsStream<TcpStream>>,
}

impl SmtpClient {
    pub fn new(server: &str, port: u16) -> Result<Self, String> {
        let stream = TcpStream::connect(format!("{}:{}", server, port))
            .map_err(|e| format!("Connection failed: {}", e))?;
        
        let mut client = SmtpClient {
            stream,
            tls_stream: None,
        };
        
        let mut buf = [0u8; 4096];
        let n = client.stream.read(&mut buf).unwrap_or(0);
        println!("[SMTP] {}", String::from_utf8_lossy(&buf[..n]).trim());
        
        Ok(client)
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
    
    pub fn starttls(&mut self) -> Result<(), String> {
        let r = self.send_command("STARTTLS")?;
        if !r.contains("220") {
            return Err(format!("STARTTLS failed: {}", r));
        }
        
        let connector = native_tls::TlsConnector::new()
            .map_err(|e| format!("TLS error: {}", e))?;
        
        let tls = connector
            .connect("smtp.gmail.com", self.stream.try_clone().map_err(|e| e.to_string())?)
            .map_err(|e| format!("TLS handshake failed: {}", e))?;
        
        self.tls_stream = Some(tls);
        Ok(())
    }
    
    pub fn auth(&mut self, user: &str, pass: &str) -> Result<(), String> {
        use base64::Engine;
        
        let _ = self.send_command("EHLO localhost");
        self.starttls()?;
        let _ = self.send_command("EHLO localhost");
        
        let r = self.send_command("AUTH LOGIN")?;
        if !r.contains("334") {
            return Err(format!("AUTH LOGIN failed: {}", r));
        }
        
        let user_b64 = base64::engine::general_purpose::STANDARD.encode(user);
        let _ = self.send_command(&user_b64)?;
        
        let pass_b64 = base64::engine::general_purpose::STANDARD.encode(pass);
        let r = self.send_command(&pass_b64)?;
        
        if r.contains("235") {
            Ok(())
        } else {
            Err(format!("Auth failed: {}", r))
        }
    }
    
    pub fn send_email(&mut self, from: &str, to: &str, subject: &str, body: &str) -> Result<(), String> {
        use base64::Engine;
        
        let _ = self.send_command(&format!("MAIL FROM:<{}>", from));
        
        let r = self.send_command(&format!("RCPT TO:<{}>", to))?;
        if !r.contains("250") {
            return Err(format!("RCPT failed: {}", r));
        }
        
        let r = self.send_command("DATA")?;
        if !r.contains("354") {
            return Err(format!("DATA failed: {}", r));
        }
        
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
        if r.contains("250") {
            Ok(())
        } else {
            Err(format!("Send failed: {}", r))
        }
    }
    
    pub fn quit(&mut self) {
        let _ = self.send_command("QUIT");
    }
}
