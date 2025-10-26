use api::create_order;
use axum::routing::{get, post};
use axum::{response::IntoResponse, Router};
use axum_prometheus::PrometheusMetricLayer;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use sysinfo::{System, SystemExt};
use tokio::sync::{mpsc, Semaphore};
use tracing::info;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
mod api;
mod clients;
mod config;
mod error;
mod models;
mod trade_validation;

/// Metrics for monitoring the orchestrator
#[derive(Clone)]
pub struct OrchestratorMetrics {
    /// Current queue length (approximate)
    queue_length: Arc<AtomicUsize>,
}

impl Default for OrchestratorMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl OrchestratorMetrics {
    #[must_use]
    pub fn new() -> Self {
        Self {
            queue_length: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn increment_queue(&self) {
        let new_len = self.queue_length.fetch_add(1, Ordering::Relaxed) + 1;
        metrics::gauge!("orchestrator_queue_length").set(new_len as f64);
    }

    pub fn decrement_queue(&self) {
        let new_len = self
            .queue_length
            .fetch_sub(1, Ordering::Relaxed)
            .saturating_sub(1);
        metrics::gauge!("orchestrator_queue_length").set(new_len as f64);
    }

    pub fn record_order_processed(&self) {
        metrics::counter!("orchestrator_orders_processed_total").increment(1);
    }

    #[must_use]
    pub fn get_queue_length(&self) -> usize {
        self.queue_length.load(Ordering::Relaxed)
    }
}

#[derive(Clone)]
pub struct AppState {
    pub order_queue: Arc<mpsc::UnboundedSender<(uuid::Uuid, crate::models::CreateOrderRequest)>>,
    pub user_client: Arc<clients::UserClient>,
    pub order_client: Arc<clients::OrderClient>,
    pub market_data_client: Arc<clients::MarketDataClient>,
    pub api_gateway_client: Arc<clients::ApiGatewayClient>,
    pub trade_validator: Arc<trade_validation::TradeValidator>,
    pub metrics: Arc<OrchestratorMetrics>,
    pub concurrency_limiter: Arc<Semaphore>,
}

#[derive(OpenApi)]
#[openapi(paths(), components(schemas()), tags())]
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
    dotenvy::dotenv().ok();
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_default())
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration
    let config = config::Config::from_env();

    // Initialize service clients
    let user_client = Arc::new(clients::UserClient::new(&config.user_service_url));
    let order_client = Arc::new(clients::OrderClient::new(&config.order_service_url));
    let market_data_client = Arc::new(clients::MarketDataClient::new(
        &config.market_data_service_url,
    ));

    // NEW: API Gateway client for cached price lookups
    let api_gateway_client = Arc::new(clients::ApiGatewayClient::new(&config.api_gateway_url));
    info!(
        "Using API Gateway at {} for cached price lookups",
        config.api_gateway_url
    );

    // Initialize trade validator - now uses API Gateway for price lookups
    let trade_validator = Arc::new(trade_validation::TradeValidator::new(
        user_client.clone(),
        api_gateway_client.clone(), // Changed from market_data_client
        trade_validation::ValidationConfig::default(),
    ));

    // Create order queue with larger capacity for recovery
    // Using unbounded channel to handle large number of pending orders on startup
    let (tx, rx) = mpsc::unbounded_channel();

    // Initialize metrics
    let metrics = Arc::new(OrchestratorMetrics::new());

    // Create concurrency limiter for order processing
    // Limit concurrent order processing to prevent overwhelming downstream services
    // Reduced from 1000 to 300 for better stability at high throughput
    let concurrency_limiter = Arc::new(Semaphore::new(300));

    let app_state = AppState {
        order_queue: Arc::new(tx.clone()),
        user_client: user_client.clone(),
        order_client: order_client.clone(),
        market_data_client: market_data_client.clone(),
        api_gateway_client: api_gateway_client.clone(),
        trade_validator: trade_validator.clone(),
        metrics: metrics.clone(),
        concurrency_limiter: concurrency_limiter.clone(),
    };

    // Spawn background processor BEFORE recovering orders
    let processor_state = Arc::new(app_state.clone());
    tokio::spawn(async move {
        info!("Order processor started and ready to process orders");
        api::order_processor(rx, processor_state).await;
    });

    // Give the processor a moment to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Recover pending orders from order service on startup
    info!("Recovering pending orders from order service...");
    if let Ok(pending_orders) = order_client.get_pending_orders().await {
        info!("Found {} pending orders to process", pending_orders.len());
        let mut enqueued = 0;
        for order in pending_orders {
            // Re-create the order request from the order response
            let order_request = models::CreateOrderRequest {
                user_id: order.client_id, // Use the client_id from the order as user_id
                symbol: order.symbol.clone(),
                quantity: order.quantity,
                order_side: order.order_side.clone(),
                order_type: order.order_type.clone(),
                price: match &order.order_type {
                    models::OrderType::Limit { price } => Some(*price),
                    models::OrderType::Market => None,
                },
            };
            if let Err(e) = tx.send((order.client_id, order_request)) {
                tracing::error!("Failed to enqueue pending order {}: {}", order.id, e);
            } else {
                metrics.increment_queue();
                enqueued += 1;
                if enqueued % 1000 == 0 {
                    info!("Enqueued {} pending orders...", enqueued);
                }
            }
        }
        info!(
            "Successfully enqueued {} pending orders for processing",
            enqueued
        );
    } else {
        tracing::warn!("Failed to recover pending orders, continuing without recovery");
    }

    let (prometheus_layer, metric_handle) = PrometheusMetricLayer::pair();
    let app = Router::new()
        .route("/v1/orders", post(create_order))
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .route("/v1/metrics", get(|| async move { metric_handle.render() }))
        .route("/v1/metrics/system", get(system_metrics_handler))
        .layer(prometheus_layer)
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.port)).await?;
    info!(
        "Order Execution Orchestrator Service listening on {}",
        listener.local_addr()?
    );
    axum::serve(listener, app).await?;
    Ok(())
}
