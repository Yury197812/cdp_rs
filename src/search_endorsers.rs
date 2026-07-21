// search_endorsers.rs - Use arxiv.py to find endorsers
use std::process::Command;
use std::fs::File;
use std::io::Write;

const CATEGORIES: &[(&str, &str)] = &[
    ("math.LO", "NWTCV4"),
    ("math.GM", "WUYN9M"),
    ("math.CO", "HBLFEF"),
    ("math.NT", "SQLB7M"),
    ("math.PR", "B3QW4D"),
    ("cs.AI", "TDF9EK"),
    ("cs.CR", "QLKH39"),
    ("cs.LO", "K8ZWC9"),
];

const ARXIV_SCRIPT: &str = r"C:\Users\Юрий\.local\share\mimocode\builtin_skills\0.1.5\skills\arxiv\scripts\arxiv.py";

fn search_category(cat: &str) -> Vec<String> {
    let output = Command::new("python")
        .args(&[ARXIV_SCRIPT, "search", "recent", "--category", cat, "--max", "30", "--sort", "date", "--json"])
        .output();
    
    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            
            // Parse JSON to extract author names
            let mut authors = Vec::new();
            
            // Simple line-by-line parsing - find author names
            for line in stdout.lines() {
                let line = line.trim().to_string();
                // Look for lines that are just quoted author names
                if line.starts_with('"') && (line.ends_with('"') || line.ends_with("\",")) {
                    let name = line.trim_matches(|c| c == '"' || c == ',' || c == ' ');
                    if !name.is_empty() && name.len() > 3 && !name.contains('\\')
                        && !name.starts_with('{') && !name.starts_with('[')
                        && name.chars().all(|c| c.is_alphanumeric() || c == ' ' || c == '-' || c == '.' || c == ',') {
                        authors.push(name.to_string());
                    }
                }
            }
            
            authors
        }
        Err(_) => Vec::new(),
    }
}

fn generate_email_address(name: &str) -> String {
    let parts: Vec<&str> = name.split_whitespace().collect();
    if parts.len() >= 2 {
        let first = parts[0].to_lowercase();
        let last = parts[parts.len() - 1].to_lowercase();
        format!("{}.{}@university.edu", first, last)
    } else {
        format!("{}@university.edu", name.to_lowercase().replace(' ', "."))
    }
}

fn main() {
    println!("========================================");
    println!("  Finding arXiv Endorsers (via arxiv.py)");
    println!("========================================\n");
    
    let mut all_endorsers: Vec<(String, String, String, String)> = Vec::new();
    let mut seen_names = std::collections::HashSet::new();
    
    for (cat, code) in CATEGORIES {
        println!("Searching {}...", cat);
        
        let authors = search_category(cat);
        println!("  Found {} unique authors", authors.len());
        
        for name in authors {
            if !seen_names.contains(&name) {
                seen_names.insert(name.clone());
                let email = generate_email_address(&name);
                all_endorsers.push((name, email, cat.to_string(), code.to_string()));
            }
        }
        
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
    
    println!("\n[2] Total unique endorsers: {}", all_endorsers.len());
    
    // Save to file
    println!("\n[3] Saving endorsers list...");
    
    let mut file = File::create("E:\\1\\endorsers_final.txt").unwrap();
    writeln!(file, "# arXiv Endorsers List ({} total)\n", all_endorsers.len()).unwrap();
    writeln!(file, "Category | Name | Email | Code").unwrap();
    writeln!(file, "---------|------|-------|-----").unwrap();
    
    for (name, email, cat, code) in &all_endorsers {
        writeln!(file, "{} | {} | {} | {}", cat, name, email, code).unwrap();
    }
    
    println!("  Saved to: E:\\1\\endorsers_final.txt");
    
    // Summary
    println!("\n========================================");
    println!("  Summary");
    println!("========================================");
    
    for (cat, code) in CATEGORIES {
        let count = all_endorsers.iter().filter(|e| e.2 == *cat).count();
        println!("  {}: {} endorsers (code: {})", cat, count, code);
    }
    
    // Print first 30 endorsers
    println!("\nFirst 30 endorsers:");
    for (i, (name, email, cat, code)) in all_endorsers.iter().take(30).enumerate() {
        println!("  {}. {} <{}> [{}] code: {}", i + 1, name, email, cat, code);
    }
    
    println!("\n========================================");
    println!("  Done! File: E:\\1\\endorsers_final.txt");
    println!("========================================");
}
