// analysis/critic/mod.rs - Critical analysis submodule
pub mod engine;
pub mod rules;

pub use engine::Critic;
pub use rules::{CritiqueResult, CritiqueRule};
