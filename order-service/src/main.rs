mod api;
mod config;
mod event_consumer;
mod event_publisher;
mod models;
mod repository;
mod service;
mod trade_validation;

#[cfg(test)]
mod tests;

use axum::{
    Router,
    http::{HeaderValue, Method},
    response::IntoResponse,
    routing::{delete, get, post, put},
};

use axum_prometheus::PrometheusMetricLayer;
use config::DatabaseConfig;
use event_consumer::OrderEventConsumer;
use event_publisher::EventPublisher;
use service::OrderService;
use std::sync::Arc;
use sysinfo::{System, SystemExt};
use tower_http::cors::{Any, CorsLayer};
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

#[derive(Clone)]
pub struct AppState {
    order_service: Arc<OrderService>,
    event_publisher: Arc<EventPublisher>,
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
        api::update_order_execution,
    ),
    components(schemas(
        models::Order,
        models::OrderStatus,
        models::OrderSide,
        models::OrderType,
        api::CreateOrderRequest,
        api::UpdateOrderRequest,
        api::UpdateOrderExecutionRequest,
        api::OrderResponse,
        api::PaginatedOrdersResponse,
        api::PaginationInfo,
        api::HealthResponse,
    )),
    tags(
        (name = "orders", description = "Order management operations"),
        (name = "health", description = "Health check operations")
    ),
    info(
        title = "Order Service API",
        version = "1.0.0",
        description = "Service for managing order lifecycle including creation, updates, and cancellation.",
    ),
    servers(
        (url = "http://localhost:3002", description = "Local development server"),
        (url = "http://order-service:3002", description = "Docker service")
    )
)]
struct ApiDoc;

async fn system_metrics_handler() -> axum::response::Response {
    let mut sys = System::new_all();
    sys.refresh_all();
    let cpu_usage = sys
        .cpus()
        .iter()
        .map(sysinfo::CpuExt::cpu_usage)
        .sum::<f32>()
        / sys.cpus().len() as f32;
    let ram_used = sys.used_memory();
    let ram_total = sys.total_memory();
    let thread_count = sys.processes().len();
    let body = format!(
        "cpu_usage {cpu_usage}\nram_used {ram_used}\nram_total {ram_total}\nthread_count {thread_count}\n"
    );
    (
        axum::http::StatusCode::OK,
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; version=0.0.4",
        )],
        body,
    )
        .into_response()
}

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
        OrderService::new_with_config("orders", &db_config)
            .await
            .expect("Failed to initialize order service"),
    );

    let event_store_url =
        std::env::var("EVENT_STORE_URL").unwrap_or_else(|_| "http://localhost:3005".to_string());
    let event_publisher = Arc::new(EventPublisher::new(event_store_url.clone()));

    let app_state = AppState {
        order_service,
        event_publisher,
    };

    let (prometheus_layer, metric_handle) = PrometheusMetricLayer::pair();

    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6380".to_string());

    let user_service_url =
        std::env::var("USER_SERVICE_URL").unwrap_or_else(|_| "http://localhost:3001".to_string());

    let market_data_service_url = std::env::var("MARKET_DATA_SERVICE_URL")
        .unwrap_or_else(|_| "http://localhost:3003".to_string());

    let event_consumer = OrderEventConsumer::new(
        app_state.order_service.clone(),
        redis_url,
        event_store_url,
        user_service_url,
        market_data_service_url,
    );

    tokio::spawn(async move {
        if let Err(e) = event_consumer.start().await {
            error!("Event consumer error: {}", e);
        }
    });

    // Configure CORS
    let cors_layer = if config.cors_allowed_origins.is_empty() {
        tracing::info!(
            "No CORS origins configured for Order Service, allowing permissive CORS for internal service"
        );
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods([
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::DELETE,
                Method::PATCH,
                Method::OPTIONS,
            ])
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
            .allow_methods([
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::DELETE,
                Method::PATCH,
                Method::OPTIONS,
            ])
            .allow_headers([axum::http::header::CONTENT_TYPE, axum::http::header::ACCEPT])
    };

    // Build routes
    let app = Router::new()
        .route("/v1/health", get(api::health_check))
        .route("/v1/orders", post(api::create_order))
        .route("/v1/orders", get(api::get_orders))
        .route("/v1/orders/pending", get(api::get_pending_orders))
        .route("/v1/orders/{id}", get(api::get_order))
        .route("/v1/orders/{id}", put(api::update_order))
        .route("/v1/orders/{id}", delete(api::cancel_order))
        .route(
            "/v1/orders/{id}/execution",
            axum::routing::patch(api::update_order_execution),
        )
        .route("/v1/metrics", get(|| async move { metric_handle.render() }))
        .route("/v1/metrics/system", get(system_metrics_handler))
        .layer(prometheus_layer)
        .layer(cors_layer)
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .with_state(app_state);

    // Start server
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.port)).await?;
    info!("Order Service listening on {}", listener.local_addr()?);

    axum::serve(listener, app).await?;

    Ok(())
}
