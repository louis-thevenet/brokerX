mod api;
mod config;
mod event_consumer;
mod event_publisher;
mod models;
mod repository;
mod service;

#[cfg(test)]
mod tests;

use crate::event_consumer::UserEventConsumer;
use crate::event_publisher::EventPublisher;
use crate::repository::UserRepository;
use axum::{
    Router,
    http::{HeaderValue, Method},
    response::IntoResponse,
    routing::{get, post, put},
};
use axum_prometheus::PrometheusMetricLayer;
use service::UserService;
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use sysinfo::{System, SystemExt};
use tower_http::cors::{Any, CorsLayer};
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

#[derive(Clone)]
pub struct AppState {
    user_service: Arc<UserService>,
    event_publisher: Arc<EventPublisher>,
}

#[derive(OpenApi)]
#[openapi(
    paths(
        api::get_user,
        api::create_user,
        api::update_user,
        api::deposit_funds,
        api::get_holdings,
        api::update_holdings,
        api::validate_trade,
        api::get_user_by_email,
        api::verify_user_email,
        api::authenticate_user,
        api::health_check,
        api::get_subscription,
        api::upsert_subscription,
        api::delete_subscription,
        api::update_subscription_last_sent,
    ),
    components(schemas(
        models::User,
        models::Holding,
        models::MarketDataSubscription,
        models::SubscriptionRequest,
        models::SubscriptionFrequency,
        api::CreateUserRequest,
        api::UpdateUserRequest,
        api::UpdateHoldingsRequest,
        api::ValidateTradeRequest,
        api::ValidateTradeResponse,
        api::AuthenticateRequest,
        api::HealthResponse,
        api::DepositRequest,
    )),
    tags(
        (name = "users", description = "User management operations"),
        (name = "auth", description = "Authentication operations"),
        (name = "health", description = "Health check operations")
    ),
    info(
        title = "User Service API",
        version = "1.0.0",
        description = "Service for managing users, authentication, holdings, and trade validation.",
    ),
    servers(
        (url = "http://localhost:3001", description = "Local development server"),
        (url = "http://user-service:3001", description = "Docker service")
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

    // Use USER_DATABASE_URL or fall back to DATABASE_URL for user service
    let user_db_url = std::env::var("USER_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .expect("Either USER_DATABASE_URL or DATABASE_URL must be set");
    let pool = PgPoolOptions::new()
        .max_connections(600)
        .min_connections(50)
        .acquire_timeout(std::time::Duration::from_secs(2))
        .idle_timeout(std::time::Duration::from_secs(300))
        .max_lifetime(std::time::Duration::from_secs(1800))
        .connect(&user_db_url)
        .await?;
    let table_name = "users".to_string();
    let user_repo = UserRepository::new(pool, table_name);
    let user_service = Arc::new(UserService::new(user_repo));

    let event_store_url =
        std::env::var("EVENT_STORE_URL").unwrap_or_else(|_| "http://localhost:3005".to_string());
    let event_publisher = Arc::new(EventPublisher::new(event_store_url));

    let app_state = AppState {
        user_service,
        event_publisher,
    };

    let (prometheus_layer, metric_handle) = PrometheusMetricLayer::pair();

    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6380".to_string());
    let event_consumer = UserEventConsumer::new(
        app_state.user_service.clone(),
        app_state.event_publisher.clone(),
        redis_url,
    );

    tokio::spawn(async move {
        if let Err(e) = event_consumer.start().await {
            error!("Event consumer error: {}", e);
        }
    });

    // Configure CORS
    let cors_layer = if config.cors_allowed_origins.is_empty() {
        tracing::info!(
            "No CORS origins configured for User Service, allowing permissive CORS for internal service"
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
        .route("/v1/users", post(api::create_user))
        .route("/v1/users/{id}", get(api::get_user))
        .route("/v1/users/{id}", put(api::update_user))
        .route("/v1/users/{id}/deposit", post(api::deposit_funds))
        .route("/v1/users/{id}/withdraw", post(api::withdraw_funds))
        .route("/v1/users/{id}/holdings", get(api::get_holdings))
        .route("/v1/users/{id}/holdings", post(api::update_holdings))
        .route("/v1/users/{id}/validate-trade", post(api::validate_trade))
        .route("/v1/users/{id}/subscription", get(api::get_subscription))
        .route("/v1/users/{id}/subscription", put(api::upsert_subscription))
        .route(
            "/v1/users/{id}/subscription",
            axum::routing::delete(api::delete_subscription),
        )
        .route(
            "/v1/users/{id}/subscription/last-sent",
            axum::routing::patch(api::update_subscription_last_sent),
        )
        .route("/v1/users/by-email/{email}", get(api::get_user_by_email))
        .route("/v1/users/{id}/verify", post(api::verify_user_email))
        .route("/v1/auth/login", post(api::authenticate_user))
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .route("/v1/metrics", get(|| async move { metric_handle.render() }))
        .route("/v1/metrics/system", get(system_metrics_handler))
        .layer(prometheus_layer)
        .layer(cors_layer)
        .with_state(app_state);

    // Start server
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.port)).await?;
    info!("User Service listening on {}", listener.local_addr()?);

    axum::serve(listener, app).await?;

    Ok(())
}
