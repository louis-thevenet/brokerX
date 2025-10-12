use crate::models::{AuthError, User};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::AppState;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateUserRequest {
    pub email: String,
    pub password: String,
    pub firstname: String,
    pub surname: String,
    #[serde(default = "default_balance")]
    pub initial_balance: f64,
}

fn default_balance() -> f64 {
    0.0
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UpdateUserRequest {
    pub email: Option<String>,
    pub firstname: Option<String>,
    pub surname: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AuthenticateRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AuthenticateResponse {
    pub authenticated: bool,
    pub message: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct HealthResponse {
    pub status: String,
    pub service: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Health check endpoint
#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "Service is healthy", body = HealthResponse)
    ),
    tag = "health"
)]
pub async fn health_check() -> impl IntoResponse {
    Json(HealthResponse {
        status: "healthy".to_string(),
        service: "user-service".to_string(),
        timestamp: chrono::Utc::now(),
    })
}

/// Get user by ID
#[utoipa::path(
    get,
    path = "/users/{id}",
    responses(
        (status = 200, description = "User found", body = User),
        (status = 404, description = "User not found"),
        (status = 500, description = "Internal server error")
    ),
    params(
        ("id" = Uuid, Path, description = "User ID")
    ),
    tag = "users"
)]
pub async fn get_user(
    State(state): State<AppState>,
    Path(user_id_str): Path<String>,
) -> impl IntoResponse {
    let user_id = match Uuid::parse_str(&user_id_str) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    match state.user_service.get_user(&user_id).await {
        Ok(Some(user)) => Json(user).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

/// Create a new user
#[utoipa::path(
    post,
    path = "/users",
    request_body = CreateUserRequest,
    responses(
        (status = 201, description = "User created successfully", body = Uuid),
        (status = 400, description = "Invalid request data"),
        (status = 409, description = "User already exists"),
        (status = 500, description = "Internal server error")
    ),
    tag = "users"
)]
pub async fn create_user(
    State(state): State<AppState>,
    payload: Result<Json<CreateUserRequest>, axum::extract::rejection::JsonRejection>,
) -> impl IntoResponse {
    let Json(payload) = match payload {
        Ok(json) => json,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    // Basic validation
    if !payload.email.contains('@') {
        return (StatusCode::BAD_REQUEST, "Invalid email format").into_response();
    }

    if payload.password.is_empty() || payload.firstname.is_empty() || payload.surname.is_empty() {
        return StatusCode::BAD_REQUEST.into_response();
    }

    // Check if user already exists
    if let Ok(Some(_)) = state.user_service.get_user_by_email(&payload.email).await {
        return (StatusCode::CONFLICT, "User already exists").into_response();
    }

    match state
        .user_service
        .create_user(
            payload.email,
            payload.password,
            payload.firstname,
            payload.surname,
            payload.initial_balance,
        )
        .await
    {
        Ok(user_id) => (StatusCode::CREATED, Json(user_id)).into_response(),
        Err(AuthError::UserAlreadyExists) => {
            (StatusCode::CONFLICT, "User already exists").into_response()
        }
        Err(AuthError::WeakPassword) => {
            (StatusCode::BAD_REQUEST, "Password too weak").into_response()
        }
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

/// Update user information
#[utoipa::path(
    put,
    path = "/users/{id}",
    request_body = UpdateUserRequest,
    responses(
        (status = 200, description = "User updated successfully", body = User),
        (status = 404, description = "User not found"),
        (status = 400, description = "Invalid request data"),
        (status = 500, description = "Internal server error")
    ),
    params(
        ("id" = Uuid, Path, description = "User ID")
    ),
    tag = "users"
)]
pub async fn update_user(
    State(_state): State<AppState>,
    Path(_user_id): Path<Uuid>,
    Json(_payload): Json<UpdateUserRequest>,
) -> impl IntoResponse {
    // For now, return not implemented as this requires more complex user management
    (
        StatusCode::NOT_IMPLEMENTED,
        "User updates not yet implemented in microservice",
    )
        .into_response()
}

/// Get user by email
#[utoipa::path(
    get,
    path = "/users/by-email/{email}",
    responses(
        (status = 200, description = "User found", body = User),
        (status = 404, description = "User not found"),
        (status = 500, description = "Internal server error")
    ),
    params(
        ("email" = String, Path, description = "User email")
    ),
    tag = "users"
)]
pub async fn get_user_by_email(
    State(state): State<AppState>,
    Path(email): Path<String>,
) -> impl IntoResponse {
    match state.user_service.get_user_by_email(&email).await {
        Ok(Some(user)) => Json(user).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

/// Verify user email
#[utoipa::path(
    post,
    path = "/users/{id}/verify",
    responses(
        (status = 200, description = "Email verified successfully"),
        (status = 404, description = "User not found"),
        (status = 500, description = "Internal server error")
    ),
    params(
        ("id" = Uuid, Path, description = "User ID")
    ),
    tag = "users"
)]
pub async fn verify_user_email(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
) -> impl IntoResponse {
    match state.user_service.verify_user_email(&user_id).await {
        Ok(()) => StatusCode::OK.into_response(),
        Err(AuthError::UserNotFound) => StatusCode::NOT_FOUND.into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

/// Authenticate user credentials
#[utoipa::path(
    post,
    path = "/auth/login",
    request_body = AuthenticateRequest,
    responses(
        (status = 200, description = "Authentication result", body = AuthenticateResponse),
        (status = 400, description = "Invalid request data"),
        (status = 500, description = "Internal server error")
    ),
    tag = "auth"
)]
pub async fn authenticate_user(
    State(state): State<AppState>,
    Json(payload): Json<AuthenticateRequest>,
) -> impl IntoResponse {
    match state
        .user_service
        .authenticate_user(&payload.email, &payload.password)
        .await
    {
        Ok(authenticated) => {
            let response = AuthenticateResponse {
                authenticated,
                message: if authenticated {
                    "Authentication successful".to_string()
                } else {
                    "Invalid credentials".to_string()
                },
            };
            Json(response).into_response()
        }
        Err(_) => {
            let response = AuthenticateResponse {
                authenticated: false,
                message: "Authentication error".to_string(),
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}
