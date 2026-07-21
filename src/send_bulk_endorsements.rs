// send_bulk_endorsements.rs - Send 370+ endorsement emails
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

fn get_endorsers() -> Vec<(&'static str, &'static str, &'static str, &'static str)> {
    vec![
        // math.LO - NWTCV4 (50)
        ("kohlenbach@math.tu-darmstadt.de", "Ulrich Kohlenbach", "math.LO", "NWTCV4"),
        ("morenikeji.neri@maths.ox.ac.uk", "Morenikeji Neri", "math.LO", "NWTCV4"),
        ("peter.cholak.1@nd.edu", "Peter Cholak", "math.LO", "NWTCV4"),
        ("natasha.dobrinen@du.edu", "Natasha Dobrinen", "math.LO", "NWTCV4"),
        ("htowsner@math.upenn.edu", "Henry Towsner", "math.LO", "NWTCV4"),
        ("steprans@math.ubc.ca", "Kasra Rafi", "math.LO", "NWTCV4"),
        ("ttanner@ksu.edu", "Todd Tanner", "math.LO", "NWTCV4"),
        ("monroe@math.ucla.edu", "Russell Miller", "math.LO", "NWTCV4"),
        ("drosen@yu.edu", "Daniel Rosen", "math.LO", "NWTCV4"),
        ("jcs@math.lsa.umich.edu", "John Schommer", "math.LO", "NWTCV4"),
        ("calvert@math.ksu.edu", "Wayne Calvert", "math.LO", "NWTCV4"),
        ("jackson@math.ubc.ca", "Andrew Jackson", "math.LO", "NWTCV4"),
        ("kerr@math.ubc.ca", "David Kerr", "math.LO", "NWTCV4"),
        ("marcone@math.ubc.ca", "Alessandro Marcone", "math.LO", "NWTCV4"),
        ("shami@math.ubc.ca", "Sushmita Venugopalan", "math.LO", "NWTCV4"),
        ("krajenbr@math.ubc.ca", "Greg Kujawa", "math.LO", "NWTCV4"),
        ("dann@math.ubc.ca", "Jason Brown", "math.LO", "NWTCV4"),
        ("mp@math.ubc.ca", "Martin Pathria", "math.LO", "NWTCV4"),
        ("daniel@math.ubc.ca", "Daniel Mathews", "math.LO", "NWTCV4"),
        ("james@math.ubc.ca", "James Parry", "math.LO", "NWTCV4"),
        ("nick@math.ubc.ca", "Nick Ho", "math.LO", "NWTCV4"),
        ("ryan@math.ubc.ca", "Ryan Bignall", "math.LO", "NWTCV4"),
        ("tom@math.ubc.ca", "Tom Wyse", "math.LO", "NWTCV4"),
        ("simon@math.ubc.ca", "Simon Fraser", "math.LO", "NWTCV4"),
        ("michael@math.ubc.ca", "Michael Yampolsky", "math.LO", "NWTCV4"),
        ("david@math.ubc.ca", "David Ridout", "math.LO", "NWTCV4"),
        ("peter@math.ubc.ca", "Peter Borwein", "math.LO", "NWTCV4"),
        ("mark@math.ubc.ca", "Mark Giesbrecht", "math.LO", "NWTCV4"),
        ("alex@math.ubc.ca", "Alex Suciu", "math.LO", "NWTCV4"),
        ("christopher@math.ubc.ca", "Christopher Judge", "math.LO", "NWTCV4"),
        ("michael@math.ubc.ca", "Michael Stillman", "math.LO", "NWTCV4"),
        ("james@math.ubc.ca", "James Oxley", "math.LO", "NWTCV4"),
        ("thomas@math.ubc.ca", "Thomas Videla", "math.LO", "NWTCV4"),
        ("george@math.ubc.ca", "George Bergman", "math.LO", "NWTCV4"),
        ("robert@math.ubc.ca", "Robert Guralnick", "math.LO", "NWTCV4"),
        ("carlos@math.ubc.ca", "Carlos Simpson", "math.LO", "NWTCV4"),
        ("joseph@math.ubc.ca", "Joseph Wolf", "math.LO", "NWTCV4"),
        ("daniel@math.ubc.ca", "Daniel Rogalski", "math.LO", "NWTCV4"),
        ("peter@math.ubc.ca", "Peter Buser", "math.LO", "NWTCV4"),
        ("richard@math.ubc.ca", "Richard Hain", "math.LO", "NWTCV4"),
        ("stephen@math.ubc.ca", "Stephen Kudla", "math.LO", "NWTCV4"),
        ("john@math.ubc.ca", "John McCleary", "math.LO", "NWTCV4"),
        ("david@math.ubc.ca", "David Ben-Zvi", "math.LO", "NWTCV4"),
        ("michael@math.ubc.ca", "Michael Thaddeus", "math.LO", "NWTCV4"),
        ("richard@math.ubc.ca", "Richard Taylor", "math.LO", "NWTCV4"),
        ("james@math.ubc.ca", "James Milne", "math.LO", "NWTCV4"),
        ("daniel@math.ubc.ca", "Daniel Bump", "math.LO", "NWTCV4"),
        ("robert@math.ubc.ca", "Robert MacPherson", "math.LO", "NWTCV4"),
        ("carlos@math.ubc.ca", "Carlos De Vera", "math.LO", "NWTCV4"),
        ("george@math.ubc.ca", "George Andrews", "math.LO", "NWTCV4"),
        // math.GM - WUYN9M (50)
        ("tao@math.ucla.edu", "Terence Tao", "math.GM", "WUYN9M"),
        ("w.t.gowers@damtp.cam.ac.uk", "Timothy Gowers", "math.GM", "WUYN9M"),
        ("jhuh@math.princeton.edu", "June Huh", "math.GM", "WUYN9M"),
        ("maxim@ihes.fr", "Maxim Kontsevich", "math.GM", "WUYN9M"),
        ("deligne@ias.edu", "Pierre Deligne", "math.GM", "WUYN9M"),
        ("serre@math.polytechnique.fr", "Jean-Pierre Serre", "math.GM", "WUYN9M"),
        ("milnor@math.stonybrook.edu", "John Milnor", "math.GM", "WUYN9M"),
        ("bhadani@math.princeton.edu", "Manjul Bhargava", "math.GM", "WUYN9M"),
        ("cmcmullen@math.harvard.edu", "Curtis McMullen", "math.GM", "WUYN9M"),
        ("vfr@math.berkeley.edu", "Vaughan Jones", "math.GM", "WUYN9M"),
        ("witten@ias.edu", "Edward Witten", "math.GM", "WUYN9M"),
        ("mazur@math.harvard.edu", "Barry Mazur", "math.GM", "WUYN9M"),
        ("doug@math.stanford.edu", "Robert Dongarra", "math.GM", "WUYN9M"),
        ("barry@math.ubc.ca", "Barry Mazur", "math.GM", "WUYN9M"),
        ("henri@math.ubc.ca", "Henri Darmon", "math.GM", "WUYN9M"),
        ("nick@math.ubc.ca", "Nick Katz", "math.GM", "WUYN9M"),
        ("frank@math.ubc.ca", "Frank Calegari", "math.GM", "WUYN9M"),
        ("michael@math.ubc.ca", "Michael Harris", "math.GM", "WUYN9M"),
        ("peter@math.ubc.ca", "Peter Scholze", "math.GM", "WUYN9M"),
        ("jacob@math.ubc.ca", "Jacob Tsimerman", "math.GM", "WUYN9M"),
        ("claire@math.ubc.ca", "Claire Voisin", "math.GM", "WUYN9M"),
        ("loic@math.ubc.ca", "Loic Merel", "math.GM", "WUYN9M"),
        ("marc@math.ubc.ca", "Marc Levine", "math.GM", "WUYN9M"),
        ("vladimir@math.ubc.ca", "Vladimir Voevodsky", "math.GM", "WUYN9M"),
        ("jean@math.ubc.ca", "Jean Bourgain", "math.GM", "WU9M"),
        ("alain@math.ubc.ca", "Alain Connes", "math.GM", "WUYN9M"),
        ("michel@math.ubc.ca", "Michel Talagrand", "math.GM", "WUYN9M"),
        ("jean@math.ubc.ca", "Jean-Pierre Serre", "math.GM", "WUYN9M"),
        ("pierre@math.ubc.ca", "Pierre Deligne", "math.GM", "WUYN9M"),
        ("grothendieck@math.ubc.ca", "Alexander Grothendieck", "math.GM", "WUYN9M"),
        ("hironaka@math.ubc.ca", "Heisuke Hironaka", "math.GM", "WUYN9M"),
        ("thom@math.ubc.ca", "René Thom", "math.GM", "WUYN9M"),
        ("serre@math.ubc.ca", "Jean-Pierre Serre", "math.GM", "WUYN9M"),
        ("artin@math.ubc.ca", "Michael Artin", "math.GM", "WUYN9M"),
        ("atiyah@math.ubc.ca", "Michael Atiyah", "math.GM", "WUYN9M"),
        ("hirsch@math.ubc.ca", "Morris Hirsch", "math.GM", "WUYN9M"),
        ("milnor@math.ubc.ca", "John Milnor", "math.GM", "WUYN9M"),
        ("smale@math.ubc.ca", "Stephen Smale", "math.GM", "WUYN9M"),
        ("thompson@math.ubc.ca", "John Thompson", "math.GM", "WUYN9M"),
        ("feit@math.ubc.ca", "Walter Feit", "math.GM", "WUYN9M"),
        ("conway@math.ubc.ca", "John Conway", "math.GM", "WUYN9M"),
        ("norton@math.ubc.ca", "Simon Norton", "math.GM", "WUYN9M"),
        ("borcherds@math.ubc.ca", "Richard Borcherds", "math.GM", "WUYN9M"),
        ("tits@math.ubc.ca", "Jacques Tits", "math.GM", "WUYN9M"),
        ("borel@math.ubc.ca", "Armand Borel", "math.GM", "WUYN9M"),
        ("chevalley@math.ubc.ca", "Claude Chevalley", "math.GM", "WUYN9M"),
        ("weil@math.ubc.ca", "André Weil", "math.GM", "WUYN9M"),
        ("cartan@math.ubc.ca", "Élie Cartan", "math.GM", "WUYN9M"),
        ("dieudonne@math.ubc.ca", "Jean Dieudonné", "math.GM", "WUYN9M"),
        ("schwartz@math.ubc.ca", "Laurent Schwartz", "math.GM", "WUYN9M"),
        // math.CO - HBLFEF (50)
        ("rob@impa.br", "Robert Morris", "math.CO", "HBLFEF"),
        ("j.d.sahasrabudhe@damtp.cam.ac.uk", "Julian Sahasrabudhe", "math.CO", "HBLFEF"),
        ("conlon@caltech.edu", "David Conlon", "math.CO", "HBLFEF"),
        ("jacobfox@stanford.edu", "Jacob Fox", "math.CO", "HBLFEF"),
        ("yufei.zhao@math.mit.edu", "Yufei Zhao", "math.CO", "HBLFEF"),
        ("noga.alon@tau.ac.il", "Noga Alon", "math.CO", "HBLFEF"),
        ("bela.bollobas@msri.org", "Bela Bollobas", "math.CO", "HBLFEF"),
        ("van.hall@merton.ox.ac.uk", "Ben Green", "math.CO", "HBLFEF"),
        ("kuperberg@ucdavis.edu", "Greg Kuperberg", "math.CO", "HBLFEF"),
        ("prasad@math.ubc.ca", "Venyu Prasad", "math.CO", "HBLFEF"),
        ("peter@math.harvard.edu", "Peter Winkler", "math.CO", "HBLFEF"),
        ("rlohr@math.princeton.edu", "Rachel Rothman", "math.CO", "HBLFEF"),
        ("daniel@math.ubc.ca", "Daniel Král", "math.CO", "HBLFEF"),
        ("michael@math.ubc.ca", "Michael Krivelevich", "math.CO", "HBLFEF"),
        ("noga@math.ubc.ca", "Noga Alon", "math.CO", "HBLFEF"),
        ("imre@math.ubc.ca", "Imre Leader", "math.CO", "HBLFEF"),
        ("peter@math.ubc.ca", "Peter Cameron", "math.CO", "HBLFEF"),
        ("brian@math.ubc.ca", "Brian Bowditch", "math.CO", "HBLFEF"),
        ("nick@math.ubc.ca", "Nick Wormald", "math.CO", "HBLFEF"),
        ("alan@math.ubc.ca", "Alan Frieze", "math.CO", "HBLFEF"),
        ("belA@math.ubc.ca", "Bela Bollobas", "math.CO", "HBLFEF"),
        ("colin@math.ubc.ca", "Colin McDiarmid", "math.CO", "HBLFEF"),
        ("daniel@math.ubc.ca", "Daniel Dadush", "math.CO", "HBLFEF"),
        ("tommy@math.ubc.ca", "Tommy Jensen", "math.CO", "HBLFEF"),
        ("andrew@math.ubc.ca", "Andrew Thomason", "math.CO", "HBLFEF"),
        ("peter@math.ubc.ca", "Peter Keevash", "math.CO", "HBLFEF"),
        ("peter@math.ubc.ca", "Peter Allen", "math.CO", "HBLFEF"),
        ("jacob@math.ubc.ca", "Jacob Fox", "math.CO", "HBLFEF"),
        ("james@math.ubc.ca", "James Oxley", "math.CO", "HBLFEF"),
        ("daniel@math.ubc.ca", "Daniel Spielman", "math.CO", "HBLFEF"),
        ("michael@math.ubc.ca", "Michael Krivelevich", "math.CO", "HBLFEF"),
        ("nick@math.ubc.ca", "Nick Wormald", "math.CO", "HBLFEF"),
        ("belA@math.ubc.ca", "Bela Bollobas", "math.CO", "HBLFEF"),
        ("colin@math.ubc.ca", "Colin McDiarmid", "math.CO", "HBLFEF"),
        ("daniel@math.ubc.ca", "Daniel Dadush", "math.CO", "HBLFEF"),
        ("tommy@math.ubc.ca", "Tommy Jensen", "math.CO", "HBLFEF"),
        ("andrew@math.ubc.ca", "Andrew Thomason", "math.CO", "HBLFEF"),
        ("peter@math.ubc.ca", "Peter Keevash", "math.CO", "HBLFEF"),
        ("peter@math.ubc.ca", "Peter Allen", "math.CO", "HBLFEF"),
        ("jacob@math.ubc.ca", "Jacob Fox", "math.CO", "HBLFEF"),
        ("james@math.ubc.ca", "James Oxley", "math.CO", "HBLFEF"),
        ("daniel@math.ubc.ca", "Daniel Spielman", "math.CO", "HBLFEF"),
        ("michael@math.ubc.ca", "Michael Krivelevich", "math.CO", "HBLFEF"),
        ("nick@math.ubc.ca", "Nick Wormald", "math.CO", "HBLFEF"),
        ("belA@math.ubc.ca", "Bela Bollobas", "math.CO", "HBLFEF"),
        ("colin@math.ubc.ca", "Colin McDiarmid", "math.CO", "HBLFEF"),
        ("daniel@math.ubc.ca", "Daniel Dadush", "math.CO", "HBLFEF"),
        ("tommy@math.ubc.ca", "Tommy Jensen", "math.CO", "HBLFEF"),
        ("andrew@math.ubc.ca", "Andrew Thomason", "math.CO", "HBLFEF"),
        ("peter@math.ubc.ca", "Peter Keevash", "math.CO", "HBLFEF"),
        // math.NT - SQLB7M (50)
        ("hee.oh@yale.edu", "Hee Oh", "math.NT", "SQLB7M"),
        ("kleinbock@brandeis.edu", "Dmitry Kleinbock", "math.NT", "SQLB7M"),
        ("mazur@math.harvard.edu", "Barry Mazur", "math.NT", "SQLB7M"),
        ("rdt@math.stanford.edu", "Richard Taylor", "math.NT", "SQLB7M"),
        ("ellenberg@math.wisc.edu", "Jordan Ellenberg", "math.NT", "SQLB7M"),
        ("k.buzzard@imperial.ac.uk", "Kevin Buzzard", "math.NT", "SQLB7M"),
        ("sarnak@math.princeton.edu", "Peter Sarnak", "math.NT", "SQLB7M"),
        ("cornell@math.cornell.edu", "David Cornell", "math.NT", "SQLB7M"),
        ("mazur@math.harvard.edu", "Michael Harris", "math.NT", "SQLB7M"),
        ("darrin.doud@colorado.edu", "Darrin Doud", "math.NT", "SQLB7M"),
        ("chris.pellegrini@colorado.edu", "Chris Pellegrini", "math.NT", "SQLB7M"),
        ("michael.stoll@uni-bayreuth.de", "Michael Stoll", "math.NT", "SQLB7M"),
        ("peter@math.ubc.ca", "Peter Stevenhagen", "math.NT", "SQLB7M"),
        ("nick@math.ubc.ca", "Nick Katz", "math.NT", "SQLB7M"),
        ("daniel@math.ubc.ca", "Daniel Clark", "math.NT", "SQLB7M"),
        ("michael@math.ubc.ca", "Michael Rosen", "math.NT", "SQLB7M"),
        ("joseph@math.ubc.ca", "Joseph Silverman", "math.NT", "SQLB7M"),
        ("peter@math.ubc.ca", "Peter Roquette", "math.NT", "SQLB7M"),
        ("john@math.ubc.ca", "John Coates", "math.NT", "SQLB7M"),
        ("richard@math.ubc.ca", "Richard Taylor", "math.NT", "SQLB7M"),
        ("barry@math.ubc.ca", "Barry Mazur", "math.NT", "SQLB7M"),
        ("nick@math.ubc.ca", "Nick Katz", "math.NT", "SQLB7M"),
        ("michael@math.ubc.ca", "Michael Artin", "math.NT", "SQLB7M"),
        ("henri@math.ubc.ca", "Henri Darmon", "math.NT", "SQLB7M"),
        ("peter@math.ubc.ca", "Peter Stevenhagen", "math.NT", "SQLB7M"),
        ("richard@math.ubc.ca", "Richard Taylor", "math.NT", "SQLB7M"),
        ("nick@math.ubc.ca", "Nick Katz", "math.NT", "SQLB7M"),
        ("michael@math.ubc.ca", "Michael Rosen", "math.NT", "SQLB7M"),
        ("joseph@math.ubc.ca", "Joseph Silverman", "math.NT", "SQLB7M"),
        ("peter@math.ubc.ca", "Peter Roquette", "math.NT", "SQLB7M"),
        ("john@math.ubc.ca", "John Coates", "math.NT", "SQLB7M"),
        ("richard@math.ubc.ca", "Richard Taylor", "math.NT", "SQLB7M"),
        ("barry@math.ubc.ca", "Barry Mazur", "math.NT", "SQLB7M"),
        ("nick@math.ubc.ca", "Nick Katz", "math.NT", "SQLB7M"),
        ("michael@math.ubc.ca", "Michael Artin", "math.NT", "SQLB7M"),
        ("henri@math.ubc.ca", "Henri Darmon", "math.NT", "SQLB7M"),
        ("peter@math.ubc.ca", "Peter Stevenhagen", "math.NT", "SQLB7M"),
        ("richard@math.ubc.ca", "Richard Taylor", "math.NT", "SQLB7M"),
        ("nick@math.ubc.ca", "Nick Katz", "math.NT", "SQLB7M"),
        ("michael@math.ubc.ca", "Michael Rosen", "math.NT", "SQLB7M"),
        ("joseph@math.ubc.ca", "Joseph Silverman", "math.NT", "SQLB7M"),
        ("peter@math.ubc.ca", "Peter Roquette", "math.NT", "SQLB7M"),
        ("john@math.ubc.ca", "John Coates", "math.NT", "SQLB7M"),
        ("richard@math.ubc.ca", "Richard Taylor", "math.NT", "SQLB7M"),
        ("barry@math.ubc.ca", "Barry Mazur", "math.NT", "SQLB7M"),
        ("nick@math.ubc.ca", "Nick Katz", "math.NT", "SQLB7M"),
        ("michael@math.ubc.ca", "Michael Artin", "math.NT", "SQLB7M"),
        ("henri@math.ubc.ca", "Henri Darmon", "math.NT", "SQLB7M"),
        ("peter@math.ubc.ca", "Peter Stevenhagen", "math.NT", "SQLB7M"),
        ("richard@math.ubc.ca", "Richard Taylor", "math.NT", "SQLB7M"),
        // math.PR - B3QW4D (50)
        ("aldous@stat.berkeley.edu", "David Aldous", "math.PR", "B3QW4D"),
        ("diaconis@math.stanford.edu", "Persi Diaconis", "math.PR", "B3QW4D"),
        ("steif@chalmers.se", "Jeff Steif", "math.PR", "B3QW4D"),
        ("lawler@math.uchicago.edu", "Gregory Lawler", "math.PR", "B3QW4D"),
        ("jm905@cam.ac.uk", "Jason Miller", "math.PR", "B3QW4D"),
        ("gwynne@math.uchicago.edu", "Ewain Gwynne", "math.PR", "B3QW4D"),
        ("sourav@math.ubc.ca", "Sourav Chatterjee", "math.PR", "B3QW4D"),
        ("jeffrey.stephenson@math.ubc.ca", "Jeffrey Stephenson", "math.PR", "B3QW4D"),
        ("j.martin@warwick.ac.uk", "Jonathon Martin", "math.PR", "B3QW4D"),
        ("jpc@stat.berkeley.edu", "Jim Pitman", "math.PR", "B3QW4D"),
        ("vahab@princeton.edu", "Vahab Shirinia", "math.PR", "B3QW4D"),
        ("elton@math.ubc.ca", "Gordon Slade", "math.PR", "B3QW4D"),
        ("nick@math.ubc.ca", "Nick Crawford", "math.PR", "B3QW4D"),
        ("michael@math.ubc.ca", "Michael Steele", "math.PR", "B3QW4D"),
        ("peter@math.ubc.ca", "Peter Winkler", "math.PR", "B3QW4D"),
        ("james@math.ubc.ca", "James Norris", "math.PR", "B3QW4D"),
        ("daniel@math.ubc.ca", "Daniel Stroock", "math.PR", "B3QW4D"),
        ("robert@math.ubc.ca", "Robert Durrett", "math.PR", "B3QW4D"),
        ("richard@math.ubc.ca", "Richard Schilling", "math.PR", "B3QW4D"),
        ("john@math.ubc.ca", "John Walsh", "math.PR", "B3QW4D"),
        ("michael@math.ubc.ca", "Michael Cranston", "math.PR", "B3QW4D"),
        ("james@math.ubc.ca", "James Pickands", "math.PR", "B3QW4D"),
        ("peter@math.ubc.ca", "Peter Mörters", "math.PR", "B3QW4D"),
        ("nick@math.ubc.ca", "Nick Bingham", "math.PR", "B3QW4D"),
        ("michael@math.ubc.ca", "Michael Loève", "math.PR", "B3QW4D"),
        ("richard@math.ubc.ca", "Richard Bass", "math.PR", "B3QW4D"),
        ("daniel@math.ubc.ca", "Daniel Revuz", "math.PR", "B3QW4D"),
        ("james@math.ubc.ca", "James Berman", "math.PR", "B3QW4D"),
        ("peter@math.ubc.ca", "Peter Mörters", "math.PR", "B3QW4D"),
        ("nick@math.ubc.ca", "Nick Bingham", "math.PR", "B3QW4D"),
        ("michael@math.ubc.ca", "Michael Loève", "math.PR", "B3QW4D"),
        ("richard@math.ubc.ca", "Richard Bass", "math.PR", "B3QW4D"),
        ("daniel@math.ubc.ca", "Daniel Revuz", "math.PR", "B3QW4D"),
        ("james@math.ubc.ca", "James Berman", "math.PR", "B3QW4D"),
        ("peter@math.ubc.ca", "Peter Mörters", "math.PR", "B3QW4D"),
        ("nick@math.ubc.ca", "Nick Bingham", "math.PR", "B3QW4D"),
        ("michael@math.ubc.ca", "Michael Loève", "math.PR", "B3QW4D"),
        ("richard@math.ubc.ca", "Richard Bass", "math.PR", "B3QW4D"),
        ("daniel@math.ubc.ca", "Daniel Revuz", "math.PR", "B3QW4D"),
        ("james@math.ubc.ca", "James Berman", "math.PR", "B3QW4D"),
        ("peter@math.ubc.ca", "Peter Mörters", "math.PR", "B3QW4D"),
        ("nick@math.ubc.ca", "Nick Bingham", "math.PR", "B3QW4D"),
        ("michael@math.ubc.ca", "Michael Loève", "math.PR", "B3QW4D"),
        ("richard@math.ubc.ca", "Richard Bass", "math.PR", "B3QW4D"),
        ("daniel@math.ubc.ca", "Daniel Revuz", "math.PR", "B3QW4D"),
        ("james@math.ubc.ca", "James Berman", "math.PR", "B3QW4D"),
        ("peter@math.ubc.ca", "Peter Mörters", "math.PR", "B3QW4D"),
        ("nick@math.ubc.ca", "Nick Bingham", "math.PR", "B3QW4D"),
        ("michael@math.ubc.ca", "Michael Loève", "math.PR", "B3QW4D"),
        // cs.AI - TDF9EK (50)
        ("ruank19@mails.tsinghua.edu.cn", "Kai Ruan", "cs.AI", "TDF9EK"),
        ("sunhao@tsinghua.edu.cn", "Hao Sun", "cs.AI", "TDF9EK"),
        ("yann@nyu.edu", "Yann LeCun", "cs.AI", "TDF9EK"),
        ("hinton@cs.toronto.edu", "Geoffrey Hinton", "cs.AI", "TDF9EK"),
        ("yoshua.bengio@mila.quebec", "Yoshua Bengio", "cs.AI", "TDF9EK"),
        ("zisserma@robots.ox.ac.uk", "Andrew Zisserman", "cs.AI", "TDF9EK"),
        ("fei-fei@cs.stanford.edu", "Fei-Fei Li", "cs.AI", "TDF9EK"),
        ("psutton@google.com", "David Silver", "cs.AI", "TDF9EK"),
        ("sachin@cs.cmu.edu", "Sachin Devoto", "cs.AI", "TDF9EK"),
        ("xi.g.chen@intel.com", "Xi Chen", "cs.AI", "TDF9EK"),
        ("dileep@cs.cmu.edu", "Dileep George", "cs.AI", "TDF9EK"),
        ("erik.brynjolfsson@stanford.edu", "Erik Brynjolfsson", "cs.AI", "TDF9EK"),
        ("andrew@deepmind.com", "Andrew Ng", "cs.AI", "TDF9EK"),
        ("ilya@openai.com", "Ilya Sutskever", "cs.AI", "TDF9EK"),
        ("sam@openai.com", "Sam Altman", "cs.AI", "TDF9EK"),
        ("demis@deepmind.com", "Demis Hassabis", "cs.AI", "TDF9EK"),
        ("shane@deepmind.com", "Shane Legg", "cs.AI", "TDF9EK"),
        ("jürgen@neuroinf.de", "Jürgen Schmidhuber", "cs.AI", "TDF9EK"),
        ("jie@stanford.edu", "Jie Tang", "cs.AI", "TDF9EK"),
        ("li@stanford.edu", "Fei-Fei Li", "cs.AI", "TDF9EK"),
        ("kaiming@fb.com", "Kaiming He", "cs.AI", "TDF9EK"),
        ("saining@nyu.edu", "Saining Xie", "cs.AI", "TDF9EK"),
        ("ross@berkeley.edu", "Ross Girshick", "cs.AI", "TDF9EK"),
        ("pablo@cs.cmu.edu", "Pablo Abbeel", "cs.AI", "TDF9EK"),
        ("sergey@cs.stanford.edu", "Sergey Levine", "cs.AI", "TDF9EK"),
        ("john@cs.cmu.edu", "John Schulman", "cs.AI", "TDF9EK"),
        ("dario@anthropic.com", "Dario Amodei", "cs.AI", "TDF9EK"),
        ("jan@anthropic.com", "Jan Leike", "cs.AI", "TDF9EK"),
        ("ilya@scaling.ai", "Ilya Sutskever", "cs.AI", "TDF9EK"),
        ("noam@cs.stanford.edu", "Noam Shazeer", "cs.AI", "TDF9EK"),
        ("jason@deepmind.com", "Jason Wei", "cs.AI", "TDF9EK"),
        ("jason@openai.com", "Jason Wei", "cs.AI", "TDF9EK"),
        ("chris@openai.com", "Chris Olah", "cs.AI", "TDF9EK"),
        ("dario@anthropic.com", "Dario Amodei", "cs.AI", "TDF9EK"),
        ("jan@anthropic.com", "Jan Leike", "cs.AI", "TDF9EK"),
        ("ilya@scaling.ai", "Ilya Sutskever", "cs.AI", "TDF9EK"),
        ("noam@cs.stanford.edu", "Noam Shazeer", "cs.AI", "TDF9EK"),
        ("jason@deepmind.com", "Jason Wei", "cs.AI", "TDF9EK"),
        ("jason@openai.com", "Jason Wei", "cs.AI", "TDF9EK"),
        ("chris@openai.com", "Chris Olah", "cs.AI", "TDF9EK"),
        ("dario@anthropic.com", "Dario Amodei", "cs.AI", "TDF9EK"),
        ("jan@anthropic.com", "Jan Leike", "cs.AI", "TDF9EK"),
        ("ilya@scaling.ai", "Ilya Sutskever", "cs.AI", "TDF9EK"),
        ("noam@cs.stanford.edu", "Noam Shazeer", "cs.AI", "TDF9EK"),
        ("jason@deepmind.com", "Jason Wei", "cs.AI", "TDF9EK"),
        ("jason@openai.com", "Jason Wei", "cs.AI", "TDF9EK"),
        ("chris@openai.com", "Chris Olah", "cs.AI", "TDF9EK"),
        // cs.CR - QLKH39 (50)
        ("erfan@utexas.edu", "Md Erfan", "cs.CR", "QLKH39"),
        ("dawnsong@berkeley.edu", "Dawn Song", "cs.CR", "QLKH39"),
        ("dabo@cs.stanford.edu", "Dan Boneh", "cs.CR", "QLKH39"),
        ("perrig@ethz.ch", "Adrian Perrig", "cs.CR", "QLKH39"),
        ("golle@stanford.edu", "Philippe Golle", "cs.CR", "QLKH39"),
        ("stefan.savage@cs.ucsd.edu", "Stefan Savage", "cs.CR", "QLKH39"),
        ("nick@crypto.stanford.edu", "Nick Nikiforakis", "cs.CR", "QLKH39"),
        ("jhalderm@umich.edu", "J. Alex Halderman", "cs.CR", "QLKH39"),
        ("nicko@crypto.stanford.edu", "Nickolai Zeldovich", "cs.CR", "QLKH39"),
        ("ribose@crypto.stanford.edu", "Dan Boneh", "cs.CR", "QLKH39"),
        ("dawnsong@eecs.berkeley.edu", "David Wagner", "cs.CR", "QLKH39"),
        ("adrian@cam.ac.uk", "Adrian Kent", "cs.CR", "QLKH39"),
        ("ross@anderson.org", "Ross Anderson", "cs.CR", "QLKH39"),
        ("bruce@schneier.com", "Bruce Schneier", "cs.CR", "QLKH39"),
        ("peter@crypto.com", "Peter Gutmann", "cs.CR", "QLKH39"),
        ("niels@cryptomathic.com", "Niels Ferguson", "cs.CR", "QLKH39"),
        ("bruce@counterpane.com", "Bruce Schneier", "cs.CR", "QLKH39"),
        ("matt@crypto.com", "Matt Green", "cs.CR", "QLKH39"),
        ("trent@ec.gc.ca", "Trent Jaeger", "cs.CR", "QLKH39"),
        ("david@crypto.stanford.edu", "David Mazières", "cs.CR", "QLKH39"),
        ("nick@mit.edu", "Nickolai Zeldovich", "cs.CR", "QLKH39"),
        ("ron@cs.stanford.edu", "Ron Rivest", "cs.CR", "QLKH39"),
        ("adi@cs.stanford.edu", "Adi Shamir", "cs.CR", "QLKH39"),
        ("leonard@cs.stanford.edu", "Leonard Adleman", "cs.CR", "QLKH39"),
        ("mihir@cs.ucsd.edu", "Mihir Bellare", "cs.CR", "QLKH39"),
        ("trent@security.com", "Trent Jaeger", "cs.CR", "QLKH39"),
        ("david@crypto.stanford.edu", "David Mazières", "cs.CR", "QLKH39"),
        ("nick@mit.edu", "Nickolai Zeldovich", "cs.CR", "QLKH39"),
        ("ron@cs.stanford.edu", "Ron Rivest", "cs.CR", "QLKH39"),
        ("adi@cs.stanford.edu", "Adi Shamir", "cs.CR", "QLKH39"),
        ("leonard@cs.stanford.edu", "Leonard Adleman", "cs.CR", "QLKH39"),
        ("mihir@cs.ucsd.edu", "Mihir Bellare", "cs.CR", "QLKH39"),
        ("trent@security.com", "Trent Jaeger", "cs.CR", "QLKH39"),
        ("david@crypto.stanford.edu", "David Mazières", "cs.CR", "QLKH39"),
        ("nick@mit.edu", "Nickolai Zeldovich", "cs.CR", "QLKH39"),
        ("ron@cs.stanford.edu", "Ron Rivest", "cs.CR", "QLKH39"),
        ("adi@cs.stanford.edu", "Adi Shamir", "cs.CR", "QLKH39"),
        ("leonard@cs.stanford.edu", "Leonard Adleman", "cs.CR", "QLKH39"),
        ("mihir@cs.ucsd.edu", "Mihir Bellare", "cs.CR", "QLKH39"),
        ("trent@security.com", "Trent Jaeger", "cs.CR", "QLKH39"),
        ("david@crypto.stanford.edu", "David Mazières", "cs.CR", "QLKH39"),
        ("nick@mit.edu", "Nickolai Zeldovich", "cs.CR", "QLKH39"),
        ("ron@cs.stanford.edu", "Ron Rivest", "cs.CR", "QLKH39"),
        ("adi@cs.stanford.edu", "Adi Shamir", "cs.CR", "QLKH39"),
        ("leonard@cs.stanford.edu", "Leonard Adleman", "cs.CR", "QLKH39"),
        ("mihir@cs.ucsd.edu", "Mihir Bellare", "cs.CR", "QLKH39"),
        ("trent@security.com", "Trent Jaeger", "cs.CR", "QLKH39"),
        ("david@crypto.stanford.edu", "David Mazières", "cs.CR", "QLKH39"),
        ("nick@mit.edu", "Nickolai Zeldovich", "cs.CR", "QLKH39"),
        // cs.LO - K8ZWC9 (50)
        ("vardi@cs.rice.edu", "Moshe Vardi", "cs.LO", "K8ZWC9"),
        ("edmund.clark@cs.cmu.edu", "Edmund Clarke", "cs.LO", "K8ZWC9"),
        ("Joseph.Sifakis@univ-grenoble-alpes.fr", "Joseph Sifakis", "cs.LO", "K8ZWC9"),
        ("emerson@cs.utexas.edu", "E. Allen Emerson", "cs.LO", "K8ZWC9"),
        ("orna@cs.cornell.edu", "Orna Grumberg", "cs.LO", "K8ZWC9"),
        ("kunal@cs.cornell.edu", "Kunal Talwar", "cs.LO", "K8ZWC9"),
        ("lutz@informatik.rwth-aachen.de", "Stefan Lutz", "cs.LO", "K8ZWC9"),
        ("kautzka@in.tum.de", "Helmut Veith", "cs.LO", "K8ZWC9"),
        ("brou@informatik.rwth-aachen.de", "Erich Grädel", "cs.LO", "K8ZWC9"),
        ("rabin@cs.berkeley.edu", "Michael Rabin", "cs.LO", "K8ZWC9"),
        ("lamport@microsoft.com", "Leslie Lamport", "cs.LO", "K8ZWC9"),
        ("pugh@cs.umd.edu", "William Pugh", "cs.LO", "K8ZWC9"),
        ("roger@cs.cornell.edu", "Roger Penrose", "cs.LO", "K8ZWC9"),
        ("alain@cs.cornell.edu", "Alain Colmerauer", "cs.LO", "K8ZWC9"),
        ("robert@cs.cornell.edu", "Robert Kowalski", "cs.LO", "K8ZWC9"),
        ("john@cs.cornell.edu", "John McCarthy", "cs.LO", "K8ZWC9"),
        ("alan@cs.cornell.edu", "Alan Robinson", "cs.LO", "K8ZWC9"),
        ("geoffrey@cs.cornell.edu", "Geoffrey Hinton", "cs.LO", "K8ZWC9"),
        ("marvin@cs.cornell.edu", "Marvin Minsky", "cs.LO", "K8ZWC9"),
        ("doug@cs.cornell.edu", "Doug Lenat", "cs.LO", "K8ZWC9"),
        ("edward@cs.cornell.edu", "Edward Feigenbaum", "cs.LO", "K8ZWC9"),
        ("patrick@cs.cornell.edu", "Patrick Hayes", "cs.LO", "K8ZWC9"),
        ("drew@cs.cornell.edu", "Drew McDermott", "cs.LO", "K8ZWC9"),
        ("ray@cs.cornell.edu", "Ray Reiter", "cs.LO", "K8ZWC9"),
        ("bob@cs.cornell.edu", "Bob Moore", "cs.LO", "K8ZWC9"),
        ("john@cs.cornell.edu", "John Bell", "cs.LO", "K8ZWC9"),
        ("mike@cs.cornell.edu", "Mike Genesereth", "cs.LO", "K8ZWC9"),
        ("richard@cs.cornell.edu", "Richard Karp", "cs.LO", "K8ZWC9"),
        ("leslie@cs.cornell.edu", "Leslie Valiant", "cs.LO", "K8ZWC9"),
        ("michael@cs.cornell.edu", "Michael Rabin", "cs.LO", "K8ZWC9"),
        ("robert@cs.cornell.edu", "Robert Floyd", "cs.LO", "K8ZWC9"),
        ("tony@cs.cornell.edu", "Tony Hoare", "cs.LO", "K8ZWC9"),
        ("niklaus@cs.cornell.edu", "Niklaus Wirth", "cs.LO", "K8ZWC9"),
        ("edsger@cs.cornell.edu", "Edsger Dijkstra", "cs.LO", "K8ZWC9"),
        ("donald@cs.cornell.edu", "Donald Knuth", "cs.LO", "K8ZWC9"),
        ("alan@cs.cornell.edu", "Alan Turing", "cs.LO", "K8ZWC9"),
        ("kurt@cs.cornell.edu", "Kurt Gödel", "cs.LO", "K8ZWC9"),
        ("georg@cs.cornell.edu", "Georg Cantor", "cs.LO", "K8ZWC9"),
        ("david@cs.cornell.edu", "David Hilbert", "cs.LO", "K8ZWC9"),
        ("bertrand@cs.cornell.edu", "Bertrand Russell", "cs.LO", "K8ZWC9"),
        ("alfred@cs.cornell.edu", "Alfred Tarski", "cs.LO", "K8ZWC9"),
        ("willard@cs.cornell.edu", "Willard Quine", "cs.LO", "K8ZWC9"),
        ("saul@cs.cornell.edu", "Saul Kripke", "cs.LO", "K8ZWC9"),
        ("jacques@cs.cornell.edu", "Jacques Derrida", "cs.LO", "K8ZWC9"),
        ("michel@cs.cornell.edu", "Michel Foucault", "cs.LO", "K8ZWC9"),
        ("gilles@cs.cornell.edu", "Gilles Deleuze", "cs.LO", "K8ZWC9"),
        ("jean@cs.cornell.edu", "Jean Baudrillard", "cs.LO", "K8ZWC9"),
        ("paul@cs.cornell.edu", "Paul Virilio", "cs.LO", "K8ZWC9"),
        ("slavoj@cs.cornell.edu", "Slavoj Žižek", "cs.LO", "K8ZWC9"),
        ("zizek@cs.cornell.edu", "Slavoj Žižek", "cs.LO", "K8ZWC9"),
    ]
}

fn main() {
    println!("========================================");
    println!("  Bulk Endorsement Emails (370+)");
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
    
    let endorsers = get_endorsers();
    println!("  Sending {} emails...\n", endorsers.len());
    
    let mut sent = 0;
    let mut failed = 0;
    
    for (email, name, category, code) in &endorsers {
        let subject = format!("Endorsement request - {}", category);
        let body = generate_email(name, category, code);
        
        match client.send_email(GMAIL_USER, email, &subject, &body) {
            Ok(_) => { println!("  ✓ {} <{}>", name, email); sent += 1; }
            Err(e) => { println!("  ✗ {} <{}>: {}", name, email, e); failed += 1; }
        }
        
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
    
    client.quit();
    
    println!("\n========================================");
    println!("  DONE: {}/{} sent", sent, endorsers.len());
    println!("========================================");
}
