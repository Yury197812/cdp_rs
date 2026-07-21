// send_endorsement_emails.rs - Send endorsement emails via Gmail SMTP
use native_tls::TlsStream;
use std::io::{Read, Write};
use std::net::TcpStream;
use base64::Engine;

const SMTP_SERVER: &str = "smtp.gmail.com";
const SMTP_PORT: u16 = 587;
const GMAIL_USER: &str = "apohob5@gmail.com";
const GMAIL_PASS: &str = "zkpsgveafmrnldrt"; // App Password

struct SmtpClient {
    stream: TcpStream,
    tls_stream: Option<TlsStream<TcpStream>>,
    tag: u32,
}

impl SmtpClient {
    fn new(server: &str, port: u16) -> Result<Self, String> {
        let stream = TcpStream::connect(format!("{}:{}", server, port))
            .map_err(|e| format!("Connection failed: {}", e))?;
        
        let mut client = SmtpClient {
            stream,
            tls_stream: None,
            tag: 0,
        };
        
        // Read greeting
        let greeting = client.read_response()?;
        println!("[S] {}", greeting.trim());
        
        Ok(client)
    }
    
    fn send_command(&mut self, command: &str) -> Result<String, String> {
        self.tag += 1;
        let full_command = format!("{}\r\n", command);
        
        if let Some(ref mut tls) = self.tls_stream {
            tls.write_all(full_command.as_bytes()).map_err(|e| e.to_string())?;
        } else {
            self.stream.write_all(full_command.as_bytes()).map_err(|e| e.to_string())?;
        }
        
        self.read_response()
    }
    
    fn read_response(&mut self) -> Result<String, String> {
        let mut response = String::new();
        let mut buffer = [0u8; 4096];
        
        loop {
            let n = if let Some(ref mut tls) = self.tls_stream {
                tls.read(&mut buffer).map_err(|e| e.to_string())?
            } else {
                self.stream.read(&mut buffer).map_err(|e| e.to_string())?
            };
            
            if n == 0 { break; }
            
            let chunk = String::from_utf8_lossy(&buffer[..n]);
            response.push_str(&chunk);
            
            if response.ends_with("\r\n") { break; }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        
        Ok(response)
    }
    
    fn starttls(&mut self) -> Result<(), String> {
        let response = self.send_command("STARTTLS")?;
        println!("[S] {}", response.trim());
        
        if !response.contains("220") {
            return Err(format!("STARTTLS failed: {}", response));
        }
        
        // Upgrade to TLS
        let connector = native_tls::TlsConnector::new()
            .map_err(|e| format!("TLS error: {}", e))?;
        
        let domain = SMTP_SERVER;
        let tls_stream = connector.connect(domain, self.stream.try_clone()
            .map_err(|e| format!("Clone failed: {}", e))?)
            .map_err(|e| format!("TLS handshake failed: {}", e))?;
        
        self.tls_stream = Some(tls_stream);
        Ok(())
    }
    
    fn auth_login(&mut self, user: &str, pass: &str) -> Result<(), String> {
        // Send EHLO
        let response = self.send_command("EHLO localhost")?;
        println!("[S] {}", response.trim());
        
        // Start TLS
        self.starttls()?;
        
        // Send EHLO again after TLS
        let response = self.send_command("EHLO localhost")?;
        println!("[S] {}", response.trim());
        
        // AUTH LOGIN
        let response = self.send_command("AUTH LOGIN")?;
        println!("[S] {}", response.trim());
        
        // Send username (base64 encoded)
        use base64::Engine;
        let user_b64 = base64::engine::general_purpose::STANDARD.encode(user);
        let response = self.send_command(&user_b64)?;
        println!("[S] {}", response.trim());
        
        // Send password (base64 encoded)
        let pass_b64 = base64::engine::general_purpose::STANDARD.encode(pass);
        let response = self.send_command(&pass_b64)?;
        println!("[S] {}", response.trim());
        
        if response.contains("235") {
            Ok(())
        } else {
            Err(format!("Auth failed: {}", response))
        }
    }
    
    fn send_email(&mut self, to: &str, subject: &str, body: &str) -> Result<(), String> {
        // MAIL FROM
        let response = self.send_command(&format!("MAIL FROM:<{}>", GMAIL_USER))?;
        println!("[S] {}", response.trim());
        
        // RCPT TO
        let response = self.send_command(&format!("RCPT TO:<{}>", to))?;
        println!("[S] {}", response.trim());
        
        // DATA
        let response = self.send_command("DATA")?;
        println!("[S] {}", response.trim());
        
        // Email content
        let email_content = format!(
            "From: Yuriy Aronov <{}>\r\n\
             To: {}\r\n\
             Subject: =?UTF-8?B?{}?=\r\n\
             MIME-Version: 1.0\r\n\
             Content-Type: text/plain; charset=UTF-8\r\n\
             Content-Transfer-Encoding: 8bit\r\n\
             \r\n\
             {}\r\n\
             .\r\n",
            GMAIL_USER,
            to,
            base64::engine::general_purpose::STANDARD.encode(subject.as_bytes()),
            body
        );
        
        let response = self.send_command(&email_content)?;
        println!("[S] {}", response.trim());
        
        if response.contains("250") {
            Ok(())
        } else {
            Err(format!("Send failed: {}", response))
        }
    }
    
    fn quit(&mut self) -> Result<(), String> {
        let _ = self.send_command("QUIT");
        Ok(())
    }
}

// Email templates for each endorser
struct EndorserEmail {
    to: String,
    name: String,
    category: String,
    code: String,
}

fn get_endorsers() -> Vec<EndorserEmail> {
    vec![
        // math.LO - Code: NWTCV4
        EndorserEmail {
            to: "kohlenbach@math.tu-darmstadt.de".to_string(),
            name: "Ulrich Kohlenbach".to_string(),
            category: "math.LO".to_string(),
            code: "NWTCV4".to_string(),
        },
        EndorserEmail {
            to: "morenikeji.neri@maths.ox.ac.uk".to_string(),
            name: "Morenikeji Neri".to_string(),
            category: "math.LO".to_string(),
            code: "NWTCV4".to_string(),
        },
        EndorserEmail {
            to: "peter.cholak.1@nd.edu".to_string(),
            name: "Peter Cholak".to_string(),
            category: "math.LO".to_string(),
            code: "NWTCV4".to_string(),
        },
        EndorserEmail {
            to: "natasha.dobrinen@du.edu".to_string(),
            name: "Natasha Dobrinen".to_string(),
            category: "math.LO".to_string(),
            code: "NWTCV4".to_string(),
        },
        EndorserEmail {
            to: "htowsner@math.upenn.edu".to_string(),
            name: "Henry Towsner".to_string(),
            category: "math.LO".to_string(),
            code: "NWTCV4".to_string(),
        },
        // math.CO - Code: HBLFEF
        EndorserEmail {
            to: "rob@impa.br".to_string(),
            name: "Robert Morris".to_string(),
            category: "math.CO".to_string(),
            code: "HBLFEF".to_string(),
        },
        EndorserEmail {
            to: "j.d.sahasrabudhe@damtp.cam.ac.uk".to_string(),
            name: "Julian Sahasrabudhe".to_string(),
            category: "math.CO".to_string(),
            code: "HBLFEF".to_string(),
        },
        EndorserEmail {
            to: "conlon@caltech.edu".to_string(),
            name: "David Conlon".to_string(),
            category: "math.CO".to_string(),
            code: "HBLFEF".to_string(),
        },
        EndorserEmail {
            to: "jacobfox@stanford.edu".to_string(),
            name: "Jacob Fox".to_string(),
            category: "math.CO".to_string(),
            code: "HBLFEF".to_string(),
        },
        EndorserEmail {
            to: "yufei.zhao@math.mit.edu".to_string(),
            name: "Yufei Zhao".to_string(),
            category: "math.CO".to_string(),
            code: "HBLFEF".to_string(),
        },
        // cs.AI - Code: TDF9EK
        EndorserEmail {
            to: "ruank19@mails.tsinghua.edu.cn".to_string(),
            name: "Kai Ruan".to_string(),
            category: "cs.AI".to_string(),
            code: "TDF9EK".to_string(),
        },
        EndorserEmail {
            to: "sunhao@tsinghua.edu.cn".to_string(),
            name: "Hao Sun".to_string(),
            category: "cs.AI".to_string(),
            code: "TDF9EK".to_string(),
        },
        EndorserEmail {
            to: "yann@nyu.edu".to_string(),
            name: "Yann LeCun".to_string(),
            category: "cs.AI".to_string(),
            code: "TDF9EK".to_string(),
        },
        EndorserEmail {
            to: "hinton@cs.toronto.edu".to_string(),
            name: "Geoffrey Hinton".to_string(),
            category: "cs.AI".to_string(),
            code: "TDF9EK".to_string(),
        },
        EndorserEmail {
            to: "yoshua.bengio@mila.quebec".to_string(),
            name: "Yoshua Bengio".to_string(),
            category: "cs.AI".to_string(),
            code: "TDF9EK".to_string(),
        },
        // cs.CR - Code: QLKH39
        EndorserEmail {
            to: "erfan@utexas.edu".to_string(),
            name: "Md Erfan".to_string(),
            category: "cs.CR".to_string(),
            code: "QLKH39".to_string(),
        },
        EndorserEmail {
            to: "dawnsong@berkeley.edu".to_string(),
            name: "Dawn Song".to_string(),
            category: "cs.CR".to_string(),
            code: "QLKH39".to_string(),
        },
        EndorserEmail {
            to: "dabo@cs.stanford.edu".to_string(),
            name: "Dan Boneh".to_string(),
            category: "cs.CR".to_string(),
            code: "QLKH39".to_string(),
        },
        // math.GM - Code: WUYN9M
        EndorserEmail {
            to: "tao@math.ucla.edu".to_string(),
            name: "Terence Tao".to_string(),
            category: "math.GM".to_string(),
            code: "WUYN9M".to_string(),
        },
        EndorserEmail {
            to: "w.t.gowers@damtp.cam.ac.uk".to_string(),
            name: "Timothy Gowers".to_string(),
            category: "math.GM".to_string(),
            code: "WUYN9M".to_string(),
        },
        EndorserEmail {
            to: "jhuh@princeton.edu".to_string(),
            name: "June Huh".to_string(),
            category: "math.GM".to_string(),
            code: "WUYN9M".to_string(),
        },
        // math.NT - Code: SQLB7M
        EndorserEmail {
            to: "wiles@math.princeton.edu".to_string(),
            name: "Andrew Wiles".to_string(),
            category: "math.NT".to_string(),
            code: "SQLB7M".to_string(),
        },
        EndorserEmail {
            to: "ribet@math.berkeley.edu".to_string(),
            name: "Ken Ribet".to_string(),
            category: "math.NT".to_string(),
            code: "SQLB7M".to_string(),
        },
        EndorserEmail {
            to: "sarnak@math.princeton.edu".to_string(),
            name: "Peter Sarnak".to_string(),
            category: "math.NT".to_string(),
            code: "SQLB7M".to_string(),
        },
        // math.PR - Code: B3QW4D
        EndorserEmail {
            to: "aldous@stat.berkeley.edu".to_string(),
            name: "David Aldous".to_string(),
            category: "math.PR".to_string(),
            code: "B3QW4D".to_string(),
        },
        EndorserEmail {
            to: "peres@microsoft.com".to_string(),
            name: "Yuval Peres".to_string(),
            category: "math.PR".to_string(),
            code: "B3QW4D".to_string(),
        },
        // cs.LO - Code: K8ZWC9
        EndorserEmail {
            to: "vardi@cs.rice.edu".to_string(),
            name: "Moshe Vardi".to_string(),
            category: "cs.LO".to_string(),
            code: "K8ZWC9".to_string(),
        },
        EndorserEmail {
            to: "edmund.clark@cs.cmu.edu".to_string(),
            name: "Edmund Clarke".to_string(),
            category: "cs.LO".to_string(),
            code: "K8ZWC9".to_string(),
        },
        EndorserEmail {
            to: "Joseph.Sifakis@univ-grenoble-alpes.fr".to_string(),
            name: "Joseph Sifakis".to_string(),
            category: "cs.LO".to_string(),
            code: "K8ZWC9".to_string(),
        },
    ]
}

fn generate_email_body(name: &str, category: &str, code: &str) -> String {
    format!(
        "Dear {},\n\n\
         I am writing to request your endorsement for my submission to arXiv.\n\n\
         Paper: \"1001 Proofs: A Rigorous Collection with Explicit Assumptions, \n\
         Dependencies, and Verification Boundaries\"\n\n\
         This is a comprehensive collection of 1001 mathematical proofs covering:\n\
         - Mathematical Logic and Foundations\n\
         - Number Theory and Algebra\n\
         - Combinatorics and Graph Theory\n\
         - Geometry and Topology\n\
         - Analysis and Probability\n\
         - Algorithms and Computational Complexity\n\n\
         Each proof includes explicit assumptions, verified dependencies, and scope boundaries.\n\n\
         To endorse this submission for the {} category, please:\n\
         1. Visit: https://arxiv.org/auth/endorse.php\n\
         2. Enter the endorsement code: {}\n\
         3. Follow the confirmation steps\n\n\
         The endorsement code for {} is: {}\n\n\
         If you have any questions about the paper, please contact me at apohob5@gmail.com.\n\n\
         Thank you for your consideration.\n\n\
         Best regards,\n\
         Yuriy Aronov\n\
         apohob5@gmail.com",
        name, category, code, category, code
    )
}

fn main() {
    println!("========================================");
    println!("  Sending Endorsement Emails (Gmail)");
    println!("========================================\n");
    
    // Connect to SMTP
    println!("[1] Connecting to Gmail SMTP...");
    let mut client = match SmtpClient::new(SMTP_SERVER, SMTP_PORT) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[ERROR] {}", e);
            return;
        }
    };
    
    // Login
    println!("\n[2] Logging in...");
    if let Err(e) = client.auth_login(GMAIL_USER, GMAIL_PASS) {
        eprintln!("[ERROR] Auth failed: {}", e);
        return;
    }
    println!("  Logged in successfully!");
    
    // Get endorsers
    let endorsers = get_endorsers();
    println!("\n[3] Sending {} emails...\n", endorsers.len());
    
    let mut sent = 0;
    let mut failed = 0;
    
    for endorser in &endorsers {
        println!("Sending to {} ({})...", endorser.name, endorser.category);
        
        let subject = format!("Endorsement request for arXiv submission - {}", endorser.category);
        let body = generate_email_body(&endorser.name, &endorser.category, &endorser.code);
        
        match client.send_email(&endorser.to, &subject, &body) {
            Ok(_) => {
                println!("  ✓ Sent to {}", endorser.to);
                sent += 1;
            }
            Err(e) => {
                println!("  ✗ Failed: {}", e);
                failed += 1;
            }
        }
        
        // Wait between emails to avoid rate limiting
        std::thread::sleep(std::time::Duration::from_secs(2));
    }
    
    // Quit
    let _ = client.quit();
    
    // Summary
    println!("\n========================================");
    println!("  Summary");
    println!("========================================");
    println!("  Sent: {}", sent);
    println!("  Failed: {}", failed);
    println!("  Total: {}", endorsers.len());
    println!("\n  Endorsement codes sent:");
    println!("    math.LO: NWTCV4");
    println!("    math.CO: HBLFEF");
    println!("    cs.AI: TDF9EK");
    println!("    cs.CR: QLKH39");
    println!("\n  Wait 1-7 days for endorsement responses.");
    println!("========================================");
}
