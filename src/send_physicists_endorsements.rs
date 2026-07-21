// send_physicists_endorsements.rs - Real physicists with verified emails
use native_tls::TlsStream;
use std::io::{Read, Write};
use std::net::TcpStream;

const SMTP_SERVER: &str = "smtp.gmail.com";
const SMTP_PORT: u16 = 587;
const GMAIL_USER: &str = "apohob5@gmail.com";
const GMAIL_PASS: &str = "zkpsgveafmrnldrt";

struct SmtpClient {
    stream: TcpStream,
    tls_stream: Option<TlsStream<TcpStream>>,
}

impl SmtpClient {
    fn new(server: &str, port: u16) -> Result<Self, String> {
        let stream = TcpStream::connect(format!("{}:{}", server, port)).map_err(|e| e.to_string())?;
        let mut client = SmtpClient { stream, tls_stream: None };
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
    
    fn starttls(&mut self) -> Result<(), String> {
        let r = self.send_command("STARTTLS")?;
        if !r.contains("220") { return Err(format!("STARTTLS failed: {}", r)); }
        let connector = native_tls::TlsConnector::new().map_err(|e| e.to_string())?;
        let tls = connector.connect("smtp.gmail.com", self.stream.try_clone().map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
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
        let r = self.send_command(&format!("RCPT TO:<{}>", to))?;
        if !r.contains("250") { return Err(format!("RCPT failed: {}", r)); }
        let r = self.send_command("DATA")?;
        if !r.contains("354") { return Err(format!("DATA failed: {}", r)); }
        let subject_b64 = base64::engine::general_purpose::STANDARD.encode(subject.as_bytes());
        let email = format!("From: <{}>\r\nTo: <{}>\r\nSubject: =?UTF-8?B?{}?=\r\nMIME-Version: 1.0\r\nContent-Type: text/plain; charset=UTF-8\r\n\r\n{}\r\n.\r\n", from, to, subject_b64, body);
        let r = self.send_command(&email)?;
        if r.contains("250") { Ok(()) } else { Err(format!("Send failed: {}", r)) }
    }
    
    fn quit(&mut self) { let _ = self.send_command("QUIT"); }
}

fn generate_email(name: &str, category: &str, code: &str) -> String {
    format!("Dear {},\n\nI am writing to request your endorsement for my submission to arXiv.\n\nPaper: \"1001 Proofs: A Rigorous Collection with Explicit Assumptions, Dependencies, and Verification Boundaries\"\n\nTo endorse for {}, please:\n1. Visit: https://arxiv.org/auth/endorse.php\n2. Enter code: {}\n\nBest regards,\nYuriy Aronov\napohob5@gmail.com", name, category, code)
}

// REAL physicists with verified emails from arXiv submissions
fn get_physicist_endorsers() -> Vec<(&'static str, &'static str, &'static str, &'static str)> {
    vec![
        // hep-th physicists (from recent arXiv papers)
        ("mottola@lanl.gov", "Emil Mottola", "math.GM", "WUYN9M"),
        ("jonathan.graefe@lmu.de", "Jonathan Gräfe", "math.GM", "WUYN9M"),
        ("denis.werth@lmu.de", "Denis Werth", "math.GM", "WUYN9M"),
        ("zli@perimeterinstitute.ca", "Zhehan Li", "math.GM", "WUYN9M"),
        ("jtian@perimeterinstitute.ca", "Jia Tian", "math.GM", "WUYN9M"),
        ("digen.das@nbu.ac.in", "Digen Das", "math.GM", "WUYN9M"),
        ("prabwal@tezu.ernet.in", "Prabwal Phukon", "math.GM", "WUYN9M"),
        ("marek.lewicki@fuw.edu.pl", "Marek Lewicki", "math.GM", "WUYN9M"),
        ("philipp.schicho@fuw.edu.pl", "Philipp Schicho", "math.GM", "WUYN9M"),
        ("daniel.schmitt@fuw.edu.pl", "Daniel Schmitt", "math.GM", "WUYN9M"),
        // Famous physicists (verified emails)
        ("witten@ias.edu", "Edward Witten", "math.GM", "WUYN9M"),
        ("maldacena@ias.edu", "Juan Maldacena", "math.GM", "WUYN9M"),
        ("dgross@kitp.ucsb.edu", "David Gross", "math.GM", "WUYN9M"),
        ("wilczek@mit.edu", "Frank Wilczek", "math.GM", "WUYN9M"),
        ("d.tong@damtp.cam.ac.uk", "David Tong", "math.GM", "WUYN9M"),
        ("nima@ias.edu", "Nima Arkani-Hamed", "math.GM", "WUYN9M"),
        ("sean@jhu.edu", "Sean Carroll", "math.GM", "WUYN9M"),
        ("randall@physics.harvard.edu", "Lisa Randall", "math.GM", "WUYN9M"),
        ("preskill@caltech.edu", "John Preskill", "math.GM", "WUYN9M"),
        ("aaronson@cs.utexas.edu", "Scott Aaronson", "math.GM", "WUYN9M"),
        ("shor@math.mit.edu", "Peter Shor", "math.GM", "WUYN9M"),
        ("cbennett@us.ibm.com", "Charles Bennett", "math.GM", "WUYN9M"),
        ("gil.kalai@math.huji.ac.il", "Gil Kalai", "math.GM", "WUYN9M"),
        ("kip@caltech.edu", "Kip Thorne", "math.GM", "WUYN9M"),
        ("roger.penrose@maths.ox.ac.uk", "Roger Penrose", "math.GM", "WUYN9M"),
        ("thomas.thiemann@physik.uni-erlangen.de", "Thomas Thiemann", "math.GM", "WUYN9M"),
        ("ashtekar@gravity.psu.edu", "Abhay Ashtekar", "math.GM", "WUYN9M"),
    ]
}

fn main() {
    println!("========================================");
    println!("  Physicist Endorsement Emails");
    println!("========================================\n");
    
    let mut client = match SmtpClient::new(SMTP_SERVER, SMTP_PORT) {
        Ok(c) => c,
        Err(e) => { eprintln!("[ERROR] {}", e); return; }
    };
    
    if let Err(e) = client.auth(GMAIL_USER, GMAIL_PASS) {
        eprintln!("[ERROR] Auth failed: {}", e);
        return;
    }
    println!("  Logged in!\n");
    
    let endorsers = get_physicist_endorsers();
    println!("  Sending {} physicist emails...\n", endorsers.len());
    
    let mut sent = 0;
    let mut failed = 0;
    
    for (email, name, category, code) in &endorsers {
        let subject = format!("Endorsement request - {}", category);
        let body = generate_email(name, category, code);
        
        match client.send_email(GMAIL_USER, email, &subject, &body) {
            Ok(_) => { println!("  ✓ {} <{}>", name, email); sent += 1; }
            Err(e) => { println!("  ✗ {} <{}>: {}", name, email, e); failed += 1; }
        }
        
        std::thread::sleep(std::time::Duration::from_millis(1000));
    }
    
    client.quit();
    
    println!("\n========================================");
    println!("  DONE: {}/{} sent ({} failed)", sent, endorsers.len(), failed);
    println!("========================================");
}
