// analysis/critic/rules.rs - Critique rules

pub struct CritiqueResult {
    pub passed: bool,
    pub issues: Vec<String>,
    pub score: u32,
}

pub enum CritiqueRule {
    LogicCheck,
    MathCheck,
    CompletenessCheck,
    ConsistencyCheck,
}

impl CritiqueRule {
    pub fn check(&self, input: &str) -> Option<String> {
        match self {
            CritiqueRule::LogicCheck => {
                if input.contains("if") && !input.contains("then") {
                    Some("Missing 'then' in conditional".to_string())
                } else {
                    None
                }
            }
            CritiqueRule::MathCheck => {
                if input.contains("=") && !input.contains("==") && !input.contains("!=") {
                    Some("Possible assignment instead of comparison".to_string())
                } else {
                    None
                }
            }
            CritiqueRule::CompletenessCheck => {
                if input.len() < 10 {
                    Some("Input too short".to_string())
                } else {
                    None
                }
            }
            CritiqueRule::ConsistencyCheck => {
                None // Placeholder
            }
        }
    }
}
