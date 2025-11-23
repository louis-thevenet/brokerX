use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;

/// Custom error type for Event Store Service
#[derive(Debug)]
pub enum EventStoreError {
    DatabaseError(sqlx::Error),
    NotFound(String),
    ValidationError(String),
    InternalError(String),
}

impl std::fmt::Display for EventStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventStoreError::DatabaseError(e) => write!(f, "Database error: {e}"),
            EventStoreError::NotFound(msg) => write!(f, "Not found: {msg}"),
            EventStoreError::ValidationError(msg) => write!(f, "Validation error: {msg}"),
            EventStoreError::InternalError(msg) => write!(f, "Internal error: {msg}"),
        }
    }
}

impl std::error::Error for EventStoreError {}

impl From<sqlx::Error> for EventStoreError {
    fn from(err: sqlx::Error) -> Self {
        EventStoreError::DatabaseError(err)
    }
}

impl From<serde_json::Error> for EventStoreError {
    fn from(err: serde_json::Error) -> Self {
        EventStoreError::InternalError(format!("JSON serialization error: {err}"))
    }
}

impl IntoResponse for EventStoreError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            EventStoreError::DatabaseError(e) => {
                tracing::error!("Database error: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Database error occurred".to_string(),
                )
            }
            EventStoreError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            EventStoreError::ValidationError(msg) => (StatusCode::BAD_REQUEST, msg),
            EventStoreError::InternalError(msg) => {
                tracing::error!("Internal error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, msg)
            }
        };

        let body = Json(json!({
            "error": error_message,
        }));

        (status, body).into_response()
    }
}

/// Alias for Result with `EventStoreError`
pub type Result<T> = std::result::Result<T, EventStoreError>;
