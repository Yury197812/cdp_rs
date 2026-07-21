// email/validator/mod.rs - Email validation submodule
pub mod dns;
pub mod smtp_check;

pub use dns::validate_email;
pub use smtp_check::smtp_verify;
