// analysis/validator/types.rs - Validation types

pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl ValidationResult {
    pub fn new() -> Self {
        ValidationResult {
            valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }
    
    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
        self.valid = false;
    }
    
    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }
    
    pub fn is_valid(&self) -> bool {
        self.valid && self.warnings.is_empty()
    }
}
