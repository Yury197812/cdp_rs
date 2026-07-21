// analysis/validator/mod.rs - Input validation submodule
pub mod types;
pub mod rules;

pub use types::ValidationResult;
pub use rules::validate_input;
