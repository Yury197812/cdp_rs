// email/smtp/mod.rs - SMTP submodule
pub mod client;
pub mod message;

pub use client::SmtpClient;
pub use message::EmailMessage;
