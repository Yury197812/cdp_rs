// analysis/critic/engine.rs - Critic engine for analysis
use super::rules::CritiqueResult;

pub struct Critic {
    rules: Vec<CritiqueRule>,
    score: CriticScore,
}

pub struct CriticScore {
    pub critic: u32,
    pub inventor: u32,
}

impl Critic {
    pub fn new() -> Self {
        Critic {
            rules: Vec::new(),
            score: CriticScore { critic: 0, inventor: 0 },
        }
    }
    
    pub fn add_rule(&mut self, rule: CritiqueRule) {
        self.rules.push(rule);
    }
    
    pub fn analyze(&mut self, input: &str) -> CritiqueResult {
        let mut issues = Vec::new();
        
        for rule in &self.rules {
            if let Some(issue) = rule.check(input) {
                issues.push(issue);
            }
        }
        
        CritiqueResult {
            passed: issues.is_empty(),
            issues,
            score: self.score.critic,
        }
    }
    
    pub fn get_score(&self) -> &CriticScore {
        &self.score
    }
    
    pub fn update_score(&mut self, points: i32) {
        if points > 0 {
            self.score.critic += points as u32;
        } else {
            self.score.inventor += (-points) as u32;
        }
    }
}

pub struct CritiqueRule {
    pub name: String,
    pub check_fn: Box<dyn Fn(&str) -> Option<String>>,
}

impl CritiqueRule {
    pub fn new(name: &str, check_fn: Box<dyn Fn(&str) -> Option<String>>) -> Self {
        CritiqueRule {
            name: name.to_string(),
            check_fn,
        }
    }
    
    pub fn check(&self, input: &str) -> Option<String> {
        (self.check_fn)(input)
    }
}
