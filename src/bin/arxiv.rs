// arxiv.rs - Combined arXiv tool
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        print_help();
        return;
    }
    
    match args[1].as_str() {
        "submit" => {
            println!("Starting arXiv submission...");
            println!("Use: arxiv_submit binary for full submission");
        }
        "codes" => {
            println!("Extracting endorsement codes from Gmail...");
            println!("Use: find_codes binary for full extraction");
        }
        "endorsers" => {
            println!("Finding potential endorsers...");
            println!("Use: search_endorsers binary for full search");
        }
        "send" => {
            println!("Sending endorsement emails...");
            println!("Use: send_endorsement_emails binary for full sending");
        }
        _ => {
            print_help();
        }
    }
}

fn print_help() {
    println!("arXiv Tool v3.0");
    println!();
    println!("Usage: arxiv <command>");
    println!();
    println!("Commands:");
    println!("  submit      Submit paper to arXiv");
    println!("  codes       Extract endorsement codes from Gmail");
    println!("  endorsers   Find potential endorsers");
    println!("  send        Send endorsement emails");
    println!();
    println!("For full functionality, use individual binaries:");
    println!("  arxiv_submit");
    println!("  find_codes");
    println!("  search_endorsers");
    println!("  send_endorsement_emails");
}
