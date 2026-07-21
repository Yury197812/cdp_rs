// orchestrator/auth/middleware.rs - Auth middleware
use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::Response,
};

use super::jwt::JwtService;

pub async fn auth_middleware(
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Get authorization header
    let auth_header = request
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));
    
    match auth_header {
        Some(token) => {
            // Verify token
            let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "secret".to_string());
            let jwt = JwtService::new(&secret);
            
            match jwt.verify_token(token) {
                Ok(claims) => {
                    // Add claims to request extensions
                    request.extensions_mut().insert(claims);
                    Ok(next.run(request).await)
                }
                Err(_) => Err(StatusCode::UNAUTHORIZED),
            }
        }
        None => Err(StatusCode::UNAUTHORIZED),
    }
}
