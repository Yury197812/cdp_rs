// orchestrator/auth/mod.rs - Authentication module
pub mod jwt;
pub mod middleware;
pub mod models;
pub mod rate_limit;

pub use jwt::JwtService;
pub use middleware::AuthMiddleware;
pub use models::{User, Role, LoginRequest, RegisterRequest};
pub use rate_limit::{RateLimiter, RateLimitConfig};
