// email/endorsements/mod.rs - Endorsement system submodule
pub mod endorsers;
pub mod sender;

pub use endorsers::{get_physicist_endorsers, Endorser};
pub use sender::send_endorsements;
