// analysis/mod.rs - Analysis and integration module
pub mod critic;
pub mod integrator;
pub mod validator;

pub use critic::{Critic, CritiqueResult};
pub use integrator::{IntegrationResult, integrate_data};
pub use validator::{validate_input, ValidationResult};
