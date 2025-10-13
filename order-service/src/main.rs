mod api;
mod config;
mod models;
mod repository;
mod service;
mod user_client;

#[cfg(test)]
mod tests;

use axum::{
    Router,
    routing::{delete, get, post, put},
};

use database_adapter::config::DatabaseConfig;
use service::OrderService;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tracing::info;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

#[derive(Clone)]
pub struct AppState {
    order_service: Arc<OrderService>,
}

#[derive(OpenApi)]
#[openapi(
    paths(
        api::health_check,
        api::get_order,
        api::create_order,
        api::get_orders,
        api::update_order,
        api::cancel_order,
    ),
    components(schemas(
        models::Order,
        models::OrderStatus,
        models::OrderSide,
        models::OrderType,
        api::CreateOrderRequest,
        api::UpdateOrderRequest,
        api::OrderResponse,
        api::PaginatedOrdersResponse,
        api::PaginationInfo,
        api::HealthResponse,
    )),
    tags(
        (name = "orders", description = "Order management operations"),
        (name = "health", description = "Health check operations")
    )
)]
struct ApiDoc;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    // Load environment variables from .env file
    dotenvy::dotenv().ok();
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_default())
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration
    let config = config::Config::from_env();

    // Use ORDER_DATABASE_URL or fall back to DATABASE_URL for order service
    let order_db_url = std::env::var("ORDER_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .expect("Either ORDER_DATABASE_URL or DATABASE_URL must be set");
    let db_config = DatabaseConfig::from_url(order_db_url);

    info!("Starting Order Service on port {}", config.port);

    // Initialize order service with dedicated database
    let order_service = Arc::new(
        OrderService::new_with_config("orders", &db_config, config.user_service_url.clone())
            .await
            .expect("Failed to initialize order service"),
    );

    let app_state = AppState { order_service };

    // Build routes
    let app = Router::new()
        .route("/health", get(api::health_check))
        .route("/orders", post(api::create_order))
        .route("/orders", get(api::get_orders))
        .route("/orders/{id}", get(api::get_order))
        .route("/orders/{id}", put(api::update_order))
        .route("/orders/{id}", delete(api::cancel_order))
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .layer(CorsLayer::permissive())
        .with_state(app_state);

    // Start server
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.port)).await?;
    info!("Order Service listening on {}", listener.local_addr()?);

    axum::serve(listener, app).await?;

    Ok(())
}
