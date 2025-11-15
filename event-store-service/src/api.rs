use axum::{response::Json, http::StatusCode};
use chrono::Utc;
use crate::models::HealthResponse;

/// Health check endpoint
#[utoipa::path(
    get,
    path = "/v1/health",
    tag = "health",
    responses(
        (status = 200, description = "Service is healthy", body = HealthResponse)
    )
)]
pub async fn health() -> Result<Json<HealthResponse>, StatusCode> {
    Ok(Json(HealthResponse {
        status: "healthy".to_string(),
        service: "event-store-service".to_string(),
        timestamp: Utc::now(),
        version: "0.1.0".to_string(),
    }))
}