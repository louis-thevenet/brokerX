use axum::{
    extract::{Request, State},
    http::header,
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, TokenData, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::web::AppState;

// JWT Secret - In production, use environment variable or secure key management
const JWT_SECRET: &[u8] = b"your_secret_key_here_change_in_production";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,   // Subject (user ID)
    pub email: String, // Username for convenience
    pub exp: i64,      // Expiration time
    pub iat: i64,      // Issued at
}

impl Claims {
    pub fn new(user_id: Uuid, username: String) -> Self {
        let now = Utc::now();
        let exp = now + Duration::hours(24); // Token expires in 24 hours

        Self {
            sub: user_id.to_string(),
            email: username,
            exp: exp.timestamp(),
            iat: now.timestamp(),
        }
    }
}

/// Generate a JWT token for the given user
pub fn create_jwt(user_id: Uuid, email: String) -> Result<String, jsonwebtoken::errors::Error> {
    let claims = Claims::new(user_id, email);
    let header = Header::default();
    let encoding_key = EncodingKey::from_secret(JWT_SECRET);

    encode(&header, &claims, &encoding_key)
}

/// Verify and decode a JWT token
pub fn verify_jwt(token: &str) -> Result<TokenData<Claims>, jsonwebtoken::errors::Error> {
    let decoding_key = DecodingKey::from_secret(JWT_SECRET);
    let validation = Validation::default();

    decode::<Claims>(token, &decoding_key, &validation)
}

/// Extract JWT token from Authorization header or cookie
fn extract_token_from_request(request: &Request) -> Option<String> {
    // Try Authorization header first (Bearer token)
    if let Some(auth_header) = request.headers().get(header::AUTHORIZATION) {
        if let Ok(auth_str) = auth_header.to_str() {
            if auth_str.starts_with("Bearer ") {
                return Some(auth_str[7..].to_string());
            }
        }
    }

    // Try cookie as fallback
    if let Some(cookie_header) = request.headers().get(header::COOKIE) {
        if let Ok(cookie_str) = cookie_header.to_str() {
            for cookie in cookie_str.split(';') {
                let cookie = cookie.trim();
                if let Some(token) = cookie.strip_prefix("token=") {
                    return Some(token.to_string());
                }
            }
        }
    }

    None
}

/// Middleware to protect routes that require authentication
pub async fn auth_middleware(
    State(app_state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    // Extract token from request
    let token = match extract_token_from_request(&request) {
        Some(token) => token,
        None => {
            // No token found, redirect to login
            return Redirect::to("/login").into_response();
        }
    };

    // Verify token
    let claims = match verify_jwt(&token) {
        Ok(token_data) => token_data.claims,
        Err(_) => {
            // Invalid token, redirect to login
            return Redirect::to("/login").into_response();
        }
    };

    // Verify user still exists in the system
    let user_id = match Uuid::parse_str(&claims.sub) {
        Ok(id) => id,
        Err(_) => return Redirect::to("/login").into_response(),
    };

    {
        let broker = app_state.lock().unwrap();
        if broker.user_repo.get(&user_id).is_none() {
            // User no longer exists, redirect to login
            return Redirect::to("/login").into_response();
        }
    }

    // Add user info to request extensions for use in handlers
    request.extensions_mut().insert(claims);

    next.run(request).await
}

/// Helper to create a cookie with the JWT token
pub fn create_auth_cookie(token: &str) -> String {
    format!(
        "token={}; HttpOnly; Secure; SameSite=Strict; Max-Age={}; Path=/",
        token,
        24 * 60 * 60 // 24 hours in seconds
    )
}

/// Helper to create a cookie that clears the auth token
pub fn create_logout_cookie() -> String {
    "token=; HttpOnly; Secure; SameSite=Strict; Max-Age=0; Path=/".to_string()
}
