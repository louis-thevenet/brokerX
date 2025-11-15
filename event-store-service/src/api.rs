use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use chrono::Utc;
use tracing::{debug, error, info};
use uuid::Uuid;

use crate::AppState;
use crate::error::{EventStoreError, Result};
use crate::models::{
    EventHistoryQuery, HealthResponse, OrderEvent, OutboxMessage, PublishEventRequest,
    PublishEventResponse, SagaState,
};

/// Health check endpoint
#[utoipa::path(
    get,
    path = "/v1/health",
    tag = "health",
    responses(
        (status = 200, description = "Service is healthy", body = HealthResponse)
    )
)]
pub async fn health() -> Result<Json<HealthResponse>> {
    Ok(Json(HealthResponse {
        status: "healthy".to_string(),
        service: "event-store-service".to_string(),
        timestamp: Utc::now(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    }))
}

/// Publish a new event to the event store
#[utoipa::path(
    post,
    path = "/v1/events",
    request_body = PublishEventRequest,
    responses(
        (status = 201, description = "Event published successfully", body = PublishEventResponse),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Internal server error")
    ),
    tag = "events"
)]
pub async fn publish_event(
    State(state): State<AppState>,
    Json(request): Json<PublishEventRequest>,
) -> Result<impl IntoResponse> {
    info!(
        "Publishing event {} for order {}",
        request.event_type, request.order_id
    );

    // Validate request
    if request.symbol.is_empty() {
        return Err(EventStoreError::ValidationError(
            "Symbol cannot be empty".to_string(),
        ));
    }

    if request.quantity <= 0 {
        return Err(EventStoreError::ValidationError(
            "Quantity must be greater than 0".to_string(),
        ));
    }

    // Save event with outbox (transactional)
    let event = state
        .repository
        .save_event_with_outbox(&request)
        .await
        .map_err(|e| {
            error!("Failed to save event: {}", e);
            EventStoreError::InternalError(format!("Failed to save event: {e}"))
        })?;

    info!(
        "Event {} published with sequence_number {}",
        event.event_id,
        event.sequence_number.unwrap_or(0)
    );

    Ok((
        StatusCode::CREATED,
        Json(PublishEventResponse {
            event_id: event.event_id,
            event_type: event.event_type,
            message: "Event published successfully".to_string(),
            sequence_number: event.sequence_number.unwrap_or(0),
        }),
    ))
}

/// Get all events for a specific order
#[utoipa::path(
    get,
    path = "/v1/events/order/{order_id}",
    params(
        ("order_id" = Uuid, Path, description = "Order ID")
    ),
    responses(
        (status = 200, description = "Order events retrieved successfully", body = Vec<OrderEvent>),
        (status = 404, description = "Order not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "events"
)]
pub async fn get_order_events(
    State(state): State<AppState>,
    Path(order_id): Path<Uuid>,
) -> Result<Json<Vec<OrderEvent>>> {
    debug!("Fetching events for order {}", order_id);

    let events = state
        .repository
        .get_order_events(order_id)
        .await
        .map_err(|e| {
            error!("Failed to fetch order events: {}", e);
            EventStoreError::InternalError(format!("Failed to fetch events: {e}"))
        })?;

    if events.is_empty() {
        return Err(EventStoreError::NotFound(format!(
            "No events found for order {order_id}"
        )));
    }

    info!("Found {} events for order {}", events.len(), order_id);
    Ok(Json(events))
}

/// Get events for a specific user (paginated)
#[utoipa::path(
    get,
    path = "/v1/events/user/{user_id}",
    params(
        ("user_id" = Uuid, Path, description = "User ID"),
        EventHistoryQuery
    ),
    responses(
        (status = 200, description = "User events retrieved successfully", body = Vec<OrderEvent>),
        (status = 500, description = "Internal server error")
    ),
    tag = "events"
)]
pub async fn get_user_events(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
    Query(query): Query<EventHistoryQuery>,
) -> Result<Json<Vec<OrderEvent>>> {
    let limit = query.limit.unwrap_or(50).min(1000);
    let offset = query.offset.unwrap_or(0);

    debug!(
        "Fetching events for user {} (limit={}, offset={})",
        user_id, limit, offset
    );

    let events = state
        .repository
        .get_user_events(user_id, limit, offset)
        .await
        .map_err(|e| {
            error!("Failed to fetch user events: {}", e);
            EventStoreError::InternalError(format!("Failed to fetch events: {e}"))
        })?;

    info!("Found {} events for user {}", events.len(), user_id);
    Ok(Json(events))
}

/// Get saga state for a specific order
#[utoipa::path(
    get,
    path = "/v1/saga/{order_id}",
    params(
        ("order_id" = Uuid, Path, description = "Order ID")
    ),
    responses(
        (status = 200, description = "Saga state retrieved successfully", body = SagaState),
        (status = 404, description = "Saga not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "saga"
)]
pub async fn get_saga_state(
    State(state): State<AppState>,
    Path(order_id): Path<Uuid>,
) -> Result<Json<SagaState>> {
    debug!("Fetching saga state for order {}", order_id);

    let saga = state
        .repository
        .get_saga_state(order_id)
        .await
        .map_err(|e| {
            error!("Failed to fetch saga state: {}", e);
            EventStoreError::InternalError(format!("Failed to fetch saga state: {}", e))
        })?;

    match saga {
        Some(saga) => {
            info!(
                "Found saga state for order {}: {:?}",
                order_id, saga.current_step
            );
            Ok(Json(saga))
        }
        None => Err(EventStoreError::NotFound(format!(
            "No saga found for order {}",
            order_id
        ))),
    }
}

/// Get unprocessed outbox messages (for debugging)
#[utoipa::path(
    get,
    path = "/v1/outbox/unprocessed",
    params(
        ("limit" = Option<i64>, Query, description = "Maximum number of messages to return (default: 100)")
    ),
    responses(
        (status = 200, description = "Unprocessed outbox messages", body = Vec<OutboxMessage>),
        (status = 500, description = "Internal server error")
    ),
    tag = "outbox"
)]
pub async fn get_unprocessed_outbox(
    State(state): State<AppState>,
    Query(query): Query<EventHistoryQuery>,
) -> Result<Json<Vec<OutboxMessage>>> {
    let limit = query.limit.unwrap_or(100).min(1000);

    debug!("Fetching unprocessed outbox messages (limit={})", limit);

    let messages = state
        .repository
        .get_unprocessed_outbox(limit)
        .await
        .map_err(|e| {
            error!("Failed to fetch outbox messages: {}", e);
            EventStoreError::InternalError(format!("Failed to fetch outbox: {}", e))
        })?;

    info!("Found {} unprocessed outbox messages", messages.len());
    Ok(Json(messages))
}
