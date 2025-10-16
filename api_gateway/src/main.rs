mod api;
mod auth;
mod clients;
mod config;
mod error;
mod mfa_service;
mod middleware;
mod models;
mod trade_validation;

use api::ApiDoc;
use axum::{
    Router,
    http::{HeaderValue, Method},
    routing::{delete, get, post, put},
};
use config::Config;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub user_client: Arc<clients::UserClient>,
    pub order_client: Arc<clients::OrderClient>,
    pub market_data_client: Arc<clients::MarketDataClient>,
    pub market_data_execution_client: Arc<clients::MarketDataExecutionClient>,
    pub mfa_service: mfa_service::MfaService,
    pub trade_validator: Arc<trade_validation::TradeValidator>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "api_gateway=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration
    dotenvy::dotenv().ok();
    let config = Arc::new(Config::from_env());

    // Initialize service clients
    let user_client = Arc::new(clients::UserClient::new(&config.user_service_url));
    let order_client = Arc::new(clients::OrderClient::new(&config.order_service_url));
    let market_data_client = Arc::new(clients::MarketDataClient::new(
        &config.market_data_service_url,
    ));
    let market_data_execution_client = Arc::new(clients::MarketDataExecutionClient::new(
        &config.market_data_service_url,
    ));

    // Initialize MFA provider with environment configuration (falls back to console logging if SMTP not configured)
    let mfa_provider = Arc::new(mfa_adapter::EmailOtpProvider::new_with_default_config());
    let mfa_service = mfa_service::MfaService::new(mfa_provider);

    // Initialize trade validator with default configuration
    let validation_config = trade_validation::ValidationConfig::default();
    let trade_validator = Arc::new(trade_validation::TradeValidator::new(
        user_client.clone(),
        market_data_client.clone(),
        validation_config,
    ));

    let app_state = AppState {
        config: config.clone(),
        user_client,
        order_client,
        market_data_client,
        market_data_execution_client,
        mfa_service,
        trade_validator,
    };

    let app = create_router(app_state);

    let listener = tokio::net::TcpListener::bind(&config.server_address).await?;

    tracing::info!("API Gateway listening on {}", config.server_address);

    axum::serve(listener, app).await?;

    Ok(())
}

fn create_router(app_state: AppState) -> Router {
    // Public routes (no authentication required)
    let public_routes = Router::new()
        .route("/v1/health", get(api::health_check))
        .route("/v1/auth/register", post(api::register))
        .route("/v1/auth/login", post(api::login))
        .route("/v1/auth/verify-login", post(api::verify_login_mfa))
        .route("/v1/auth/logout", post(api::logout))
        // Market data public endpoints
        .route("/v1/market/prices/{symbol}", get(api::get_market_price))
        .route("/v1/market/prices", get(api::get_market_prices))
        .route("/v1/market/symbols", get(api::get_market_symbols))
        .route(
            "/v1/market/prices/{symbol}/history",
            get(api::get_market_history),
        );

    // Protected routes (authentication required)
    let protected_routes = Router::new()
        .route("/v1/me", get(api::get_me))
        .route("/v1/me", put(api::update_me))
        .route("/v1/me/deposit", post(api::deposit_funds))
        .route("/v1/orders", post(api::create_order))
        .route("/v1/orders", get(api::get_my_orders))
        .route("/v1/orders/{id}", get(api::get_order))
        .route("/v1/orders/{id}", put(api::update_order))
        .route("/v1/orders/{id}", delete(api::cancel_order))
        .layer(axum::middleware::from_fn_with_state(
            app_state.clone(),
            middleware::auth_middleware,
        ));

    // Configure CORS with specific allowed origins from config
    let cors_layer = if app_state.config.cors_allowed_origins.is_empty() {
        tracing::warn!(
            "No CORS origins configured, allowing all origins (insecure for production!)"
        );
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods([
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::DELETE,
                Method::OPTIONS,
            ])
            .allow_headers(Any)
            .allow_credentials(true)
    } else {
        tracing::info!(
            "CORS configured with allowed origins: {:?}",
            app_state.config.cors_allowed_origins
        );
        let origins: Vec<HeaderValue> = app_state
            .config
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
                Method::OPTIONS,
            ])
            .allow_headers([
                axum::http::header::AUTHORIZATION,
                axum::http::header::CONTENT_TYPE,
                axum::http::header::ACCEPT,
            ])
            .allow_credentials(true)
    };

    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .layer(cors_layer)
        .with_state(app_state)
}
