mod api;
mod config;
mod execution_simulator;
mod models;
mod repository;
mod service;

#[cfg(test)]
mod tests;

use api::AppState;
use axum::{Router, routing::get, routing::post};
use repository::MarketDataRepository;
use service::MarketDataService;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tracing::{debug, error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

#[derive(OpenApi)]
#[openapi(
    paths(
        api::health_check,
        api::get_current_price,
        api::get_current_prices,
        api::get_historical_prices,
        api::get_available_symbols,
        api::initialize_market_data,
        api::simulate_price_update,
        api::execute_order,
    ),
    components(schemas(
        models::MarketPriceResponse,
        models::MarketPricesResponse,
        models::HistoricalPricesResponse,
        models::HistoricalPricesQuery,
        models::ExecuteOrderRequest,
        models::ExecuteOrderResponse,
        models::OrderSide,
        models::OrderType,
        models::ExecutionStatus,
        api::HealthResponse,
        api::MessageResponse,
        api::SymbolsResponse,
        api::InitializeResponse,
    )),
    tags(
        (name = "prices", description = "Market price operations"),
        (name = "symbols", description = "Symbol management"),
        (name = "admin", description = "Administrative operations"),
        (name = "execution", description = "Order execution simulation"),
        (name = "health", description = "Health check operations")
    )
)]
struct ApiDoc;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    dotenvy::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_default())
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = config::Config::from_env();

    // Use MARKET_DATA_DATABASE_URL or fall back to DATABASE_URL for market data service
    let market_data_db_url = std::env::var("MARKET_DATA_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .expect("Either MARKET_DATA_DATABASE_URL or DATABASE_URL must be set");

    info!("Starting Market Data Service");
    info!("Port: {}", config.port);
    info!(
        "Database: {}",
        market_data_db_url.split('@').next_back().unwrap_or("***")
    );

    // Initialize database connection using sqlx
    info!("Connecting to database...");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&market_data_db_url)
        .await?;
    info!("Database connection established");

    // Run migrations
    info!("Running database migrations...");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");
    info!("Migrations completed successfully");

    let repository = MarketDataRepository::new(pool);
    let market_data_service = Arc::new(MarketDataService::new(repository.clone()));
    let execution_simulator = Arc::new(execution_simulator::ExecutionSimulator::new(repository));

    info!("Services initialized");

    // Start background task for periodic price simulation
    info!("Starting background price simulation task (30s interval)");
    let simulation_service = market_data_service.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(3));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            interval.tick().await;
            debug!("[SCHEDULED] Running price simulation");

            match simulation_service.simulate_price_update().await {
                Ok(count) => {
                    debug!("[SIMULATION] Updated {} symbols", count);
                }
                Err(e) => {
                    error!("[SIMULATION] Error: {}", e);
                }
            }
        }
    });

    let app_state = AppState {
        market_data_service,
        execution_simulator,
    };

    // Build the router
    let app = Router::new()
        // Health check
        .route("/v1/health", get(api::health_check))
        // API routes
        .route("/v1/prices/{symbol}", get(api::get_current_price))
        .route("/v1/prices", get(api::get_current_prices))
        .route(
            "/v1/prices/{symbol}/history",
            get(api::get_historical_prices),
        )
        .route("/v1/symbols", get(api::get_available_symbols))
        .route("/v1/initialize", post(api::initialize_market_data))
        .route("/v1/simulate", post(api::simulate_price_update))
        // Order execution endpoint
        .route("/v1/execute", post(api::execute_order))
        // Swagger UI
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        // Add state and middleware
        .with_state(app_state)
        .layer(CorsLayer::permissive())
        .layer(tower_http::trace::TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.port)).await?;

    info!(
        "Market Data Service listening on {}",
        listener.local_addr()?
    );
    info!(
        "Swagger UI available at http://localhost:{}/swagger-ui",
        config.port
    );

    axum::serve(listener, app).await?;

    Ok(())
}
