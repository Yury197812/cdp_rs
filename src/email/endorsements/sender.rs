// email/endorsements/sender.rs - Endorsement email sender
use crate::email::smtp::SmtpClient;
use super::endorsers::Endorser;

pub fn send_endorsements(smtp: &mut SmtpClient, endorsers: &[Endorser], from: &str) -> (usize, usize) {
    let mut sent = 0;
    let mut failed = 0;
    
    for endorser in endorsers {
        let subject = format!("Endorsement request - {}", endorser.category);
        let body = format!(
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
            endorser.name, endorser.category, endorser.code
        );
        
        match smtp.send_email(from, &endorser.email, &subject, &body) {
            Ok(_) => {
                println!("  ✓ {} <{}>", endorser.name, endorser.email);
                sent += 1;
            }
            Err(e) => {
                println!("  ✗ {} <{}>: {}", endorser.name, endorser.email, e);
                failed += 1;
            }
        }
        
        std::thread::sleep(std::time::Duration::from_millis(1000));
    }
    
    (sent, failed)
}
