// email/endorsements/endorsers.rs - Endorser lists

pub struct Endorser {
    pub email: String,
    pub name: String,
    pub category: String,
    pub code: String,
}

pub fn get_physicist_endorsers() -> Vec<Endorser> {
    vec![
        Endorser { email: "mottola@lanl.gov".into(), name: "Emil Mottola".into(), category: "math.GM".into(), code: "WUYN9M".into() },
        Endorser { email: "jonathan.graefe@lmu.de".into(), name: "Jonathan Gräfe".into(), category: "math.GM".into(), code: "WUYN9M".into() },
        Endorser { email: "denis.werth@lmu.de".into(), name: "Denis Werth".into(), category: "math.GM".into(), code: "WUYN9M".into() },
        Endorser { email: "zli@perimeterinstitute.ca".into(), name: "Zhehan Li".into(), category: "math.GM".into(), code: "WUYN9M".into() },
        Endorser { email: "jtian@perimeterinstitute.ca".into(), name: "Jia Tian".into(), category: "math.GM".into(), code: "WUYN9M".into() },
        Endorser { email: "digen.das@nbu.ac.in".into(), name: "Digen Das".into(), category: "math.GM".into(), code: "WUYN9M".into() },
        Endorser { email: "prabwal@tezu.ernet.in".into(), name: "Prabwal Phukon".into(), category: "math.GM".into(), code: "WUYN9M".into() },
        Endorser { email: "marek.lewicki@fuw.edu.pl".into(), name: "Marek Lewicki".into(), category: "math.GM".into(), code: "WUYN9M".into() },
        Endorser { email: "philipp.schicho@fuw.edu.pl".into(), name: "Philipp Schicho".into(), category: "math.GM".into(), code: "WUYN9M".into() },
        Endorser { email: "daniel.schmitt@fuw.edu.pl".into(), name: "Daniel Schmitt".into(), category: "math.GM".into(), code: "WUYN9M".into() },
        Endorser { email: "witten@ias.edu".into(), name: "Edward Witten".into(), category: "math.GM".into(), code: "WUYN9M".into() },
        Endorser { email: "maldacena@ias.edu".into(), name: "Juan Maldacena".into(), category: "math.GM".into(), code: "WUYN9M".into() },
        Endorser { email: "dgross@kitp.ucsb.edu".into(), name: "David Gross".into(), category: "math.GM".into(), code: "WUYN9M".into() },
        Endorser { email: "wilczek@mit.edu".into(), name: "Frank Wilczek".into(), category: "math.GM".into(), code: "WUYN9M".into() },
        Endorser { email: "d.tong@damtp.cam.ac.uk".into(), name: "David Tong".into(), category: "math.GM".into(), code: "WUYN9M".into() },
        Endorser { email: "nima@ias.edu".into(), name: "Nima Arkani-Hamed".into(), category: "math.GM".into(), code: "WUYN9M".into() },
        Endorser { email: "sean@jhu.edu".into(), name: "Sean Carroll".into(), category: "math.GM".into(), code: "WUYN9M".into() },
        Endorser { email: "randall@physics.harvard.edu".into(), name: "Lisa Randall".into(), category: "math.GM".into(), code: "WUYN9M".into() },
        Endorser { email: "preskill@caltech.edu".into(), name: "John Preskill".into(), category: "math.GM".into(), code: "WUYN9M".into() },
        Endorser { email: "aaronson@cs.utexas.edu".into(), name: "Scott Aaronson".into(), category: "math.GM".into(), code: "WUYN9M".into() },
        Endorser { email: "shor@math.mit.edu".into(), name: "Peter Shor".into(), category: "math.GM".into(), code: "WUYN9M".into() },
        Endorser { email: "cbennett@us.ibm.com".into(), name: "Charles Bennett".into(), category: "math.GM".into(), code: "WUYN9M".into() },
        Endorser { email: "gil.kalai@math.huji.ac.il".into(), name: "Gil Kalai".into(), category: "math.GM".into(), code: "WUYN9M".into() },
        Endorser { email: "kip@caltech.edu".into(), name: "Kip Thorne".into(), category: "math.GM".into(), code: "WUYN9M".into() },
        Endorser { email: "roger.penrose@maths.ox.ac.uk".into(), name: "Roger Penrose".into(), category: "math.GM".into(), code: "WUYN9M".into() },
        Endorser { email: "thomas.thiemann@physik.uni-erlangen.de".into(), name: "Thomas Thiemann".into(), category: "math.GM".into(), code: "WUYN9M".into() },
        Endorser { email: "ashtekar@gravity.psu.edu".into(), name: "Abhay Ashtekar".into(), category: "math.GM".into(), code: "WUYN9M".into() },
    ]
}
