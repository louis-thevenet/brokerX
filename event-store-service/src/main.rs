use axum::{routing::get, Router};
use config::DatabaseConfig;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;
use tracing::info;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

mod api;
mod config;
mod models;

use models::{HealthResponse, MessageResponse};

#[derive(OpenApi)]
#[openapi(
    paths(api::health),
    components(schemas(HealthResponse, MessageResponse)),
    tags(
        (name = "health", description = "Health check endpoints")
    ),
    info(
        title = "Event Store Service API",
        version = "0.1.0",
        description = "Event Store Service for BrokerX+ - Event Sourcing and CQRS"
    )
)]
struct ApiDoc;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
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

    // Create application state
    let app_state = AppState { db };

    // Create router
    let app = Router::new()
        .route("/v1/health", get(api::health))
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
