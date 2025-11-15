use axum::{
    Router,
    routing::{get, post},
};
use config::DatabaseConfig;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;
use tracing::info;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

mod api;
mod config;
mod error;
mod models;
mod redis_client;
mod repository;

#[cfg(test)]
mod tests;

use models::{
    EventHistoryQuery, HealthResponse, MessageResponse, OrderEvent, OrderEventType, OrderSide,
    OrderType, OutboxMessage, PublishEventRequest, PublishEventResponse, SagaState, SagaStep,
};
use redis_client::RedisClient;
use repository::EventStoreRepository;
use std::sync::Arc;

#[derive(OpenApi)]
#[openapi(
    paths(
        api::health,
        api::publish_event,
        api::get_order_events,
        api::get_user_events,
        api::get_saga_state,
        api::get_unprocessed_outbox,
    ),
    components(schemas(
        HealthResponse,
        MessageResponse,
        PublishEventRequest,
        PublishEventResponse,
        OrderEvent,
        OrderEventType,
        OrderSide,
        OrderType,
        SagaState,
        SagaStep,
        OutboxMessage,
        EventHistoryQuery,
    )),
    tags(
        (name = "health", description = "Health check endpoints"),
        (name = "events", description = "Event publishing and retrieval"),
        (name = "saga", description = "Saga state management"),
        (name = "outbox", description = "Outbox pattern monitoring")
    ),
    info(
        title = "Event Store Service API",
        version = "0.1.0",
        description = "Event Store Service for BrokerX+ - Event Sourcing and CQRS with Saga Choreography"
    )
)]
struct ApiDoc;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub repository: EventStoreRepository,
    pub redis_client: Arc<RedisClient>,
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    // Initialize error handling and tracing
    color_eyre::install()?;
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    // Load configuration
    let db_config = DatabaseConfig::from_env();
    info!("Event Store Service starting");

    // Connect to database
    info!("Connecting to database...");
    let db = PgPoolOptions::new()
        .max_connections(db_config.max_connections)
        .connect(&db_config.database_url)
        .await?;
    info!("Database connection established");

    // Create repository
    let repository = EventStoreRepository::new(db.clone());

    // Connect to Redis
    info!("Connecting to Redis...");
    let redis_client = Arc::new(RedisClient::new(&db_config.redis_url).await?);

    // Health check Redis
    redis_client.ping().await?;
    info!("Redis connection verified");

    // Create application state
    let app_state = AppState {
        db,
        repository,
        redis_client,
    };

    // Create router
    let app = Router::new()
        .route("/v1/health", get(api::health))
        .route("/v1/events", post(api::publish_event))
        .route("/v1/events/order/{order_id}", get(api::get_order_events))
        .route("/v1/events/user/{user_id}", get(api::get_user_events))
        .route("/v1/saga/{order_id}", get(api::get_saga_state))
        .route("/v1/outbox/unprocessed", get(api::get_unprocessed_outbox))
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .layer(CorsLayer::permissive())
        .with_state(app_state);

    // Get port from environment
    let port = std::env::var("EVENT_STORE_SERVICE_PORT")
        .unwrap_or_else(|_| "3005".to_string())
        .parse::<u16>()
        .unwrap_or(3005);

    let addr = format!("0.0.0.0:{port}");
    info!("Event Store Service listening on {}", addr);

    // Start main server
    let listener = TcpListener::bind(&addr).await?;
    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
}
