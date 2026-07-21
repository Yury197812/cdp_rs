// bin/send_physicists.rs - Send endorsement emails to physicists
use cdp_rs::email::smtp::SmtpClient;
use cdp_rs::email::endorsements::{get_physicist_endorsers, send_endorsements};

const SMTP_SERVER: &str = "smtp.gmail.com";
const SMTP_PORT: u16 = 587;
const GMAIL_USER: &str = "apohob5@gmail.com";
const GMAIL_PASS: &str = "zkpsgveafmrnldrt";

fn main() {
    println!("========================================");
    println!("  Physicist Endorsement Emails");
    println!("========================================\n");
    
    // Connect to SMTP
    println!("[1] Connecting to Gmail SMTP...");
    let mut smtp = match SmtpClient::new(SMTP_SERVER, SMTP_PORT) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[ERROR] {}", e);
            return;
        }
    };
    
    // Login
    println!("[2] Logging in...");
    if let Err(e) = smtp.auth(GMAIL_USER, GMAIL_PASS) {
        eprintln!("[ERROR] Auth failed: {}", e);
        return;
    }
    println!("  Logged in!\n");
    
    // Get endorsers
    let endorsers = get_physicist_endorsers();
    println!("[3] Sending {} physicist emails...\n", endorsers.len());
    
    // Send emails
    let (sent, failed) = send_endorsements(&mut smtp, &endorsers, GMAIL_USER);
    
    // Quit
    smtp.quit();
    
    // Summary
    println!("\n========================================");
    println!("  DONE: {}/{} sent ({} failed)", sent, endorsers.len(), failed);
    println!("========================================");
}
