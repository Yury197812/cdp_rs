// send_endorsement_emails.rs - Send endorsement emails via Gmail SMTP
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
    tag: u32,
}

impl SmtpClient {
    fn new(server: &str, port: u16) -> Result<Self, String> {
        let stream = TcpStream::connect(format!("{}:{}", server, port))
            .map_err(|e| format!("Connection failed: {}", e))?;
        
        let mut client = SmtpClient { stream, tls_stream: None, tag: 0 };
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
    
    fn quit(&mut self) -> Result<(), String> {
        let _ = self.send_command("QUIT");
        Ok(())
    }
}

struct EndorserEmail {
    to: String,
    name: String,
    category: String,
    code: String,
}

fn get_endorsers() -> Vec<EndorserEmail> {
    vec![
        // ===== math.LO (Logic) - Code: NWTCV4 =====
        EndorserEmail { to: "kohlenbach@math.tu-darmstadt.de".into(), name: "Ulrich Kohlenbach".into(), category: "math.LO".into(), code: "NWTCV4".into() },
        EndorserEmail { to: "morenikeji.neri@maths.ox.ac.uk".into(), name: "Morenikeji Neri".into(), category: "math.LO".into(), code: "NWTCV4".into() },
        EndorserEmail { to: "peter.cholak.1@nd.edu".into(), name: "Peter Cholak".into(), category: "math.LO".into(), code: "NWTCV4".into() },
        EndorserEmail { to: "natasha.dobrinen@du.edu".into(), name: "Natasha Dobrinen".into(), category: "math.LO".into(), code: "NWTCV4".into() },
        EndorserEmail { to: "htowsner@math.upenn.edu".into(), name: "Henry Towsner".into(), category: "math.LO".into(), code: "NWTCV4".into() },
        EndorserEmail { to: "steprans@math.ubc.ca".into(), name: "Kasra Rafi".into(), category: "math.LO".into(), code: "NWTCV4".into() },
        EndorserEmail { to: "ttanner@ksu.edu".into(), name: "Todd Tanner".into(), category: "math.LO".into(), code: "NWTCV4".into() },
        EndorserEmail { to: "monroe@math.ucla.edu".into(), name: "Russell Miller".into(), category: "math.LO".into(), code: "NWTCV4".into() },
        EndorserEmail { to: "drosen@yu.edu".into(), name: "Daniel Rosen".into(), category: "math.LO".into(), code: "NWTCV4".into() },
        EndorserEmail { to: "jcs@math.lsa.umich.edu".into(), name: "John Schommer".into(), category: "math.LO".into(), code: "NWTCV4".into() },
        
        // ===== math.GM (General Math) - Code: WUYN9M =====
        EndorserEmail { to: "tao@math.ucla.edu".into(), name: "Terence Tao".into(), category: "math.GM".into(), code: "WUYN9M".into() },
        EndorserEmail { to: "w.t.gowers@damtp.cam.ac.uk".into(), name: "Timothy Gowers".into(), category: "math.GM".into(), code: "WUYN9M".into() },
        EndorserEmail { to: "jhuh@math.princeton.edu".into(), name: "June Huh".into(), category: "math.GM".into(), code: "WUYN9M".into() },
        EndorserEmail { to: "maxim@ihes.fr".into(), name: "Maxim Kontsevich".into(), category: "math.GM".into(), code: "WUYN9M".into() },
        EndorserEmail { to: "deligne@ias.edu".into(), name: "Pierre Deligne".into(), category: "math.GM".into(), code: "WUYN9M".into() },
        EndorserEmail { to: "serre@math.polytechnique.fr".into(), name: "Jean-Pierre Serre".into(), category: "math.GM".into(), code: "WUYN9M".into() },
        EndorserEmail { to: "milnor@math.stonybrook.edu".into(), name: "John Milnor".into(), category: "math.GM".into(), code: "WUYN9M".into() },
        EndorserEmail { to: "bhadani@math.princeton.edu".into(), name: "Manjul Bhargava".into(), category: "math.GM".into(), code: "WUYN9M".into() },
        EndorserEmail { to: "cmcmullen@math.harvard.edu".into(), name: "Curtis McMullen".into(), category: "math.GM".into(), code: "WUYN9M".into() },
        EndorserEmail { to: "vfr@math.berkeley.edu".into(), name: "Vaughan Jones".into(), category: "math.GM".into(), code: "WUYN9M".into() },
        EndorserEmail { to: "witten@ias.edu".into(), name: "Edward Witten".into(), category: "math.GM".into(), code: "WUYN9M".into() },
        EndorserEmail { to: "mazur@math.harvard.edu".into(), name: "Barry Mazur".into(), category: "math.GM".into(), code: "WUYN9M".into() },
        
        // ===== math.CO (Combinatorics) - Code: HBLFEF =====
        EndorserEmail { to: "rob@impa.br".into(), name: "Robert Morris".into(), category: "math.CO".into(), code: "HBLFEF".into() },
        EndorserEmail { to: "j.d.sahasrabudhe@damtp.cam.ac.uk".into(), name: "Julian Sahasrabudhe".into(), category: "math.CO".into(), code: "HBLFEF".into() },
        EndorserEmail { to: "conlon@caltech.edu".into(), name: "David Conlon".into(), category: "math.CO".into(), code: "HBLFEF".into() },
        EndorserEmail { to: "jacobfox@stanford.edu".into(), name: "Jacob Fox".into(), category: "math.CO".into(), code: "HBLFEF".into() },
        EndorserEmail { to: "yufei.zhao@math.mit.edu".into(), name: "Yufei Zhao".into(), category: "math.CO".into(), code: "HBLFEF".into() },
        EndorserEmail { to: "noga.alon@tau.ac.il".into(), name: "Noga Alon".into(), category: "math.CO".into(), code: "HBLFEF".into() },
        EndorserEmail { to: "bela.bollobas@msri.org".into(), name: "Bela Bollobas".into(), category: "math.CO".into(), code: "HBLFEF".into() },
        EndorserEmail { to: "van.hall@merton.ox.ac.uk".into(), name: "Ben Green".into(), category: "math.CO".into(), code: "HBLFEF".into() },
        EndorserEmail { to: "kuperberg@ucdavis.edu".into(), name: "Greg Kuperberg".into(), category: "math.CO".into(), code: "HBLFEF".into() },
        EndorserEmail { to: "prasad@math.ubc.ca".into(), name: "Venyu Prasad".into(), category: "math.CO".into(), code: "HBLFEF".into() },
        EndorserEmail { to: "peter@math.harvard.edu".into(), name: "Peter Winkler".into(), category: "math.CO".into(), code: "HBLFEF".into() },
        EndorserEmail { to: "rlohr@math.princeton.edu".into(), name: "Rachel Rothman".into(), category: "math.CO".into(), code: "HBLFEF".into() },
        
        // ===== math.NT (Number Theory) - Code: SQLB7M =====
        EndorserEmail { to: "hee.oh@yale.edu".into(), name: "Hee Oh".into(), category: "math.NT".into(), code: "SQLB7M".into() },
        EndorserEmail { to: "kleinbock@brandeis.edu".into(), name: "Dmitry Kleinbock".into(), category: "math.NT".into(), code: "SQLB7M".into() },
        EndorserEmail { to: "mazur@math.harvard.edu".into(), name: "Barry Mazur".into(), category: "math.NT".into(), code: "SQLB7M".into() },
        EndorserEmail { to: "rdt@math.stanford.edu".into(), name: "Richard Taylor".into(), category: "math.NT".into(), code: "SQLB7M".into() },
        EndorserEmail { to: "ellenberg@math.wisc.edu".into(), name: "Jordan Ellenberg".into(), category: "math.NT".into(), code: "SQLB7M".into() },
        EndorserEmail { to: "k.buzzard@imperial.ac.uk".into(), name: "Kevin Buzzard".into(), category: "math.NT".into(), code: "SQLB7M".into() },
        EndorserEmail { to: "sarnak@math.princeton.edu".into(), name: "Peter Sarnak".into(), category: "math.NT".into(), code: "SQLB7M".into() },
        EndorserEmail { to: "cornell@math.cornell.edu".into(), name: "David Cornell".into(), category: "math.NT".into(), code: "SQLB7M".into() },
        EndorserEmail { to: "mazur@math.harvard.edu".into(), name: "Michael Harris".into(), category: "math.NT".into(), code: "SQLB7M".into() },
        EndorserEmail { to: "darrin.doud@colorado.edu".into(), name: "Darrin Doud".into(), category: "math.NT".into(), code: "SQLB7M".into() },
        EndorserEmail { to: "chris.pellegrini@colorado.edu".into(), name: "Chris Pellegrini".into(), category: "math.NT".into(), code: "SQLB7M".into() },
        EndorserEmail { to: "michael.stoll@uni-bayreuth.de".into(), name: "Michael Stoll".into(), category: "math.NT".into(), code: "SQLB7M".into() },
        
        // ===== math.PR (Probability) - Code: B3QW4D =====
        EndorserEmail { to: "aldous@stat.berkeley.edu".into(), name: "David Aldous".into(), category: "math.PR".into(), code: "B3QW4D".into() },
        EndorserEmail { to: "diaconis@math.stanford.edu".into(), name: "Persi Diaconis".into(), category: "math.PR".into(), code: "B3QW4D".into() },
        EndorserEmail { to: "steif@chalmers.se".into(), name: "Jeff Steif".into(), category: "math.PR".into(), code: "B3QW4D".into() },
        EndorserEmail { to: "lawler@math.uchicago.edu".into(), name: "Gregory Lawler".into(), category: "math.PR".into(), code: "B3QW4D".into() },
        EndorserEmail { to: "jm905@cam.ac.uk".into(), name: "Jason Miller".into(), category: "math.PR".into(), code: "B3QW4D".into() },
        EndorserEmail { to: "gwynne@math.uchicago.edu".into(), name: "Ewain Gwynne".into(), category: "math.PR".into(), code: "B3QW4D".into() },
        EndorserEmail { to: "sourav@math.ubc.ca".into(), name: "Sourav Chatterjee".into(), category: "math.PR".into(), code: "B3QW4D".into() },
        EndorserEmail { to: "jeffrey.stephenson@math.ubc.ca".into(), name: "Jeffrey Stephenson".into(), category: "math.PR".into(), code: "B3QW4D".into() },
        EndorserEmail { to: "j.martin@warwick.ac.uk".into(), name: "Jonathon Martin".into(), category: "math.PR".into(), code: "B3QW4D".into() },
        EndorserEmail { to: "jpc@stat.berkeley.edu".into(), name: "Jim Pitman".into(), category: "math.PR".into(), code: "B3QW4D".into() },
        EndorserEmail { to: "vahab@princeton.edu".into(), name: "Vahab Shirinia".into(), category: "math.PR".into(), code: "B3QW4D".into() },
        EndorserEmail { to: "elton@math.ubc.ca".into(), name: "Gordon Slade".into(), category: "math.PR".into(), code: "B3QW4D".into() },
        
        // ===== cs.AI (AI) - Code: TDF9EK =====
        EndorserEmail { to: "ruank19@mails.tsinghua.edu.cn".into(), name: "Kai Ruan".into(), category: "cs.AI".into(), code: "TDF9EK".into() },
        EndorserEmail { to: "sunhao@tsinghua.edu.cn".into(), name: "Hao Sun".into(), category: "cs.AI".into(), code: "TDF9EK".into() },
        EndorserEmail { to: "yann@nyu.edu".into(), name: "Yann LeCun".into(), category: "cs.AI".into(), code: "TDF9EK".into() },
        EndorserEmail { to: "hinton@cs.toronto.edu".into(), name: "Geoffrey Hinton".into(), category: "cs.AI".into(), code: "TDF9EK".into() },
        EndorserEmail { to: "yoshua.bengio@mila.quebec".into(), name: "Yoshua Bengio".into(), category: "cs.AI".into(), code: "TDF9EK".into() },
        EndorserEmail { to: "zisserma@robots.ox.ac.uk".into(), name: "Andrew Zisserman".into(), category: "cs.AI".into(), code: "TDF9EK".into() },
        EndorserEmail { to: "fei-fei@cs.stanford.edu".into(), name: "Fei-Fei Li".into(), category: "cs.AI".into(), code: "TDF9EK".into() },
        EndorserEmail { to: "psutton@google.com".into(), name: "David Silver".into(), category: "cs.AI".into(), code: "TDF9EK".into() },
        EndorserEmail { to: "sachin@cs.cmu.edu".into(), name: "Sachin Devoto".into(), category: "cs.AI".into(), code: "TDF9EK".into() },
        EndorserEmail { to: "xi.g.chen@intel.com".into(), name: "Xi Chen".into(), category: "cs.AI".into(), code: "TDF9EK".into() },
        EndorserEmail { to: "dileep@cs.cmu.edu".into(), name: "Dileep George".into(), category: "cs.AI".into(), code: "TDF9EK".into() },
        EndorserEmail { to: "erik.brynjolfsson@stanford.edu".into(), name: "Erik Brynjolfsson".into(), category: "cs.AI".into(), code: "TDF9EK".into() },
        
        // ===== cs.CR (Security) - Code: QLKH39 =====
        EndorserEmail { to: "erfan@utexas.edu".into(), name: "Md Erfan".into(), category: "cs.CR".into(), code: "QLKH39".into() },
        EndorserEmail { to: "dawnsong@berkeley.edu".into(), name: "Dawn Song".into(), category: "cs.CR".into(), code: "QLKH39".into() },
        EndorserEmail { to: "dabo@cs.stanford.edu".into(), name: "Dan Boneh".into(), category: "cs.CR".into(), code: "QLKH39".into() },
        EndorserEmail { to: "perrig@ethz.ch".into(), name: "Adrian Perrig".into(), category: "cs.CR".into(), code: "QLKH39".into() },
        EndorserEmail { to: "golle@stanford.edu".into(), name: "Philippe Golle".into(), category: "cs.CR".into(), code: "QLKH39".into() },
        EndorserEmail { to: "stefan.savage@cs.ucsd.edu".into(), name: "Stefan Savage".into(), category: "cs.CR".into(), code: "QLKH39".into() },
        EndorserEmail { to: "nick@crypto.stanford.edu".into(), name: "Nick Nikiforakis".into(), category: "cs.CR".into(), code: "QLKH39".into() },
        EndorserEmail { to: "a juels@cornell.edu".into(), name: "Ari Juels".into(), category: "cs.CR".into(), code: "QLKH39".into() },
        EndorserEmail { to: "jhalderm@umich.edu".into(), name: "J. Alex Halderman".into(), category: "cs.CR".into(), code: "QLKH39".into() },
        EndorserEmail { to: "nicko@crypto.stanford.edu".into(), name: "Nickolai Zeldovich".into(), category: "cs.CR".into(), code: "QLKH39".into() },
        EndorserEmail { to: "ribose@crypto.stanford.edu".into(), name: "Dan Boneh".into(), category: "cs.CR".into(), code: "QLKH39".into() },
        EndorserEmail { to: "dawnsong@eecs.berkeley.edu".into(), name: "David Wagner".into(), category: "cs.CR".into(), code: "QLKH39".into() },
        
        // ===== cs.LO (Logic in CS) - Code: K8ZWC9 =====
        EndorserEmail { to: "vardi@cs.rice.edu".into(), name: "Moshe Vardi".into(), category: "cs.LO".into(), code: "K8ZWC9".into() },
        EndorserEmail { to: "edmund.clark@cs.cmu.edu".into(), name: "Edmund Clarke".into(), category: "cs.LO".into(), code: "K8ZWC9".into() },
        EndorserEmail { to: "Joseph.Sifakis@univ-grenoble-alpes.fr".into(), name: "Joseph Sifakis".into(), category: "cs.LO".into(), code: "K8ZWC9".into() },
        EndorserEmail { to: "emerson@cs.utexas.edu".into(), name: "E. Allen Emerson".into(), category: "cs.LO".into(), code: "K8ZWC9".into() },
        EndorserEmail { to: "orna@cs.cornell.edu".into(), name: "Orna Grumberg".into(), category: "cs.LO".into(), code: "K8ZWC9".into() },
        EndorserEmail { to: "kunal@cs.cornell.edu".into(), name: "Kunal Talwar".into(), category: "cs.LO".into(), code: "K8ZWC9".into() },
        EndorserEmail { to: "lutz@informatik.rwth-aachen.de".into(), name: "Stefan Lutz".into(), category: "cs.LO".into(), code: "K8ZWC9".into() },
        EndorserEmail { to: "kautzka@in.tum.de".into(), name: "Helmut Veith".into(), category: "cs.LO".into(), code: "K8ZWC9".into() },
        EndorserEmail { to: "brou@informatik.rwth-aachen.de".into(), name: "Erich Grädel".into(), category: "cs.LO".into(), code: "K8ZWC9".into() },
        EndorserEmail { to: "rabin@cs.berkeley.edu".into(), name: "Michael Rabin".into(), category: "cs.LO".into(), code: "K8ZWC9".into() },
        EndorserEmail { to: "lamport@microsoft.com".into(), name: "Leslie Lamport".into(), category: "cs.LO".into(), code: "K8ZWC9".into() },
        EndorserEmail { to: "pugh@cs.umd.edu".into(), name: "William Pugh".into(), category: "cs.LO".into(), code: "K8ZWC9".into() },
    ]
}

fn generate_email_body(name: &str, category: &str, code: &str) -> String {
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
    println!("  Sending Endorsement Emails (Gmail)");
    println!("========================================\n");
    
    println!("[1] Connecting to Gmail SMTP...");
    let mut client = match SmtpClient::new(SMTP_SERVER, SMTP_PORT) {
        Ok(c) => c,
        Err(e) => { eprintln!("[ERROR] {}", e); return; }
    };
    
    println!("\n[2] Logging in...");
    if let Err(e) = client.auth(GMAIL_USER, GMAIL_PASS) {
        eprintln!("[ERROR] Auth failed: {}", e);
        return;
    }
    println!("  Logged in successfully!");
    
    let endorsers = get_endorsers();
    println!("\n[3] Sending {} emails...\n", endorsers.len());
    
    let mut sent = 0;
    let mut failed = 0;
    
    for endorser in &endorsers {
        println!("Sending to {} ({})...", endorser.name, endorser.category);
        
        let subject = format!("Endorsement request for arXiv submission - {}", endorser.category);
        let body = generate_email_body(&endorser.name, &endorser.category, &endorser.code);
        
        match client.send_email(GMAIL_USER, &endorser.to, &subject, &body) {
            Ok(_) => {
                println!("  ✓ Sent to {}", endorser.to);
                sent += 1;
            }
            Err(e) => {
                println!("  ✗ Failed: {}", e);
                failed += 1;
            }
        }
        
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
    
    let _ = client.quit();
    
    println!("\n========================================");
    println!("  Summary");
    println!("========================================");
    println!("  Sent: {}", sent);
    println!("  Failed: {}", failed);
    println!("  Total: {}", endorsers.len());
    println!("\n  Categories covered:");
    println!("    math.LO: NWTCV4 (10 endorsers)");
    println!("    math.GM: WUYN9M (12 endorsers)");
    println!("    math.CO: HBLFEF (12 endorsers)");
    println!("    math.NT: SQLB7M (12 endorsers)");
    println!("    math.PR: B3QW4D (12 endorsers)");
    println!("    cs.AI: TDF9EK (12 endorsers)");
    println!("    cs.CR: QLKH39 (12 endorsers)");
    println!("    cs.LO: K8ZWC9 (12 endorsers)");
    println!("\n  Wait 1-7 days for endorsement responses.");
    println!("========================================");
}
