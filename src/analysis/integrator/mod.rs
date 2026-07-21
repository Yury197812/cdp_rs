// analysis/integrator/mod.rs - Data integration submodule
pub mod merger;
pub mod transformer;

pub use merger::IntegrationResult;
pub use transformer::integrate_data;
