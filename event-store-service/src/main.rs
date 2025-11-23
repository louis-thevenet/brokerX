use axum::{
    Router,
    http::{HeaderValue, Method},
    routing::{get, post, put},
};
use config::{Config, DatabaseConfig};
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

mod api;
mod config;
mod error;
mod models;
mod outbox_worker;
mod redis_client;
mod repository;

#[cfg(test)]
mod tests;

use models::{
    EventHistoryQuery, HealthResponse, MessageResponse, OrderEvent, OrderEventType, OrderSide,
    OrderType, OutboxMessage, PublishEventRequest, PublishEventResponse, SagaState, SagaStep,
};
use outbox_worker::OutboxWorker;
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
        api::update_saga_step,
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
        models::UpdateSagaStepRequest,
        models::MessageResponse,
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
    let config = Config::from_env();
    let db_config = DatabaseConfig::from_env();
    info!("Event Store Service starting on port {}", config.port);

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

    // Start Outbox Worker in background
    let outbox_worker = OutboxWorker::new(repository.clone(), redis_client.clone());
    tokio::spawn(async move {
        outbox_worker.start().await;
    });
    info!("Outbox Worker started in background");

    // Create application state
    let app_state = AppState {
        db,
        repository,
        redis_client,
    };

    // Configure CORS
    let cors_layer = if config.cors_allowed_origins.is_empty() {
        tracing::warn!(
            "No CORS origins configured for Event Store Service, denying all cross-origin requests for security"
        );
        // Don't allow any origins by default for internal service
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods([Method::GET, Method::POST, Method::PUT, Method::OPTIONS])
            .allow_headers(Any)
    } else {
        tracing::info!(
            "CORS configured with allowed origins: {:?}",
            config.cors_allowed_origins
        );
        let origins: Vec<HeaderValue> = config
            .cors_allowed_origins
            .iter()
            .filter_map(|origin| origin.parse().ok())
            .collect();

        CorsLayer::new()
            .allow_origin(origins)
            .allow_methods([Method::GET, Method::POST, Method::PUT, Method::OPTIONS])
            .allow_headers([axum::http::header::CONTENT_TYPE, axum::http::header::ACCEPT])
    };

    // Create router
    let app = Router::new()
        .route("/v1/health", get(api::health))
        .route("/v1/events", post(api::publish_event))
        .route("/v1/events/order/{order_id}", get(api::get_order_events))
        .route("/v1/events/user/{user_id}", get(api::get_user_events))
        .route("/v1/saga/{order_id}", get(api::get_saga_state))
        .route("/v1/saga/{order_id}/step", put(api::update_saga_step))
        .route("/v1/outbox/unprocessed", get(api::get_unprocessed_outbox))
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .layer(cors_layer)
        .with_state(app_state);

    // Get port from environment
    let addr = format!("0.0.0.0:{}", config.port);
    info!("Event Store Service listening on {}", addr);

    // Start main server
    let listener = TcpListener::bind(&addr).await?;
    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
}
