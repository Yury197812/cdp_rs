// email/mod.rs - Email module with submodules
pub mod smtp;
pub mod validator;
pub mod endorsements;

// Re-exports for convenience
pub use smtp::SmtpClient;
pub use validator::dns::validate_email;
pub use validator::smtp_check::smtp_verify;
pub use endorsements:: endorsers::{get_physicist_endorsers, Endorser};
pub use endorsements::sender::send_endorsements;
