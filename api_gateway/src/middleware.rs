use crate::AppState;
use crate::auth::JwtService;
use crate::error::ApiError;
use axum::{
    extract::{Request, State},
    http::HeaderMap,
    middleware::Next,
    response::Response,
};
use uuid::Uuid;

const BEARER_PREFIX: &str = "Bearer ";

pub async fn auth_middleware(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    // Extract JWT token from Authorization header
    let token = extract_token_from_headers(&headers)?;

    // Create JWT service and validate token
    let jwt_service = JwtService::new(&state.config.jwt_secret, state.config.jwt_expiry_hours);
    let user_id = jwt_service.extract_user_id(&token)?;

    // Add user ID to request extensions for use in handlers
    request.extensions_mut().insert(user_id);

    Ok(next.run(request).await)
}

fn extract_token_from_headers(headers: &HeaderMap) -> Result<String, ApiError> {
    let auth_header = headers
        .get(axum::http::header::AUTHORIZATION)
        .ok_or(ApiError::Unauthorized)?;

    let auth_str = auth_header.to_str().map_err(|_| ApiError::Unauthorized)?;

    if !auth_str.starts_with(BEARER_PREFIX) {
        return Err(ApiError::Unauthorized);
    }

    Ok(auth_str[BEARER_PREFIX.len()..].to_string())
}

// Extension trait to easily extract user ID from request extensions
// Helper struct to extract user ID from request extensions
#[derive(Clone)]
pub struct AuthenticatedUser(pub Uuid);

impl<S> axum::extract::FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<Uuid>()
            .copied()
            .map(AuthenticatedUser)
            .ok_or(ApiError::Unauthorized)
    }
}
