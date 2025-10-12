mod api;
mod auth;
mod clients;
mod config;
mod error;
mod middleware;
mod models;

use axum::{
    Router,
    http::Method,
    routing::{delete, get, post, put},
};
use config::Config;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::api::ApiDoc;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub user_client: Arc<clients::UserClient>,
    pub order_client: Arc<clients::OrderClient>,
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
    let config = Arc::new(Config::from_env()?);

    // Initialize service clients
    let user_client = Arc::new(clients::UserClient::new(&config.user_service_url));
    let order_client = Arc::new(clients::OrderClient::new(&config.order_service_url));

    let app_state = AppState {
        config: config.clone(),
        user_client,
        order_client,
    };

    // Build our application with routes
    let app = create_router(app_state);

    let listener = tokio::net::TcpListener::bind(&config.server_address).await?;

    tracing::info!("API Gateway listening on {}", config.server_address);

    axum::serve(listener, app).await?;

    Ok(())
}

fn create_router(app_state: AppState) -> Router {
    Router::new()
        // Health check
        .route("/health", get(api::health_check))
        // Authentication routes
        .route("/auth/register", post(api::register))
        .route("/auth/login", post(api::login))
        .route("/auth/logout", post(api::logout))
        // User profile routes (authenticated)
        .route("/me", get(api::get_me))
        .route("/me", put(api::update_me))
        .route("/me/verify-email", post(api::verify_email))
        .route("/me/deposit", post(api::deposit_funds))
        // Order routes (authenticated)
        .route("/orders", post(api::create_order))
        .route("/orders", get(api::get_my_orders))
        .route("/orders/:id", get(api::get_order))
        .route("/orders/:id", put(api::update_order))
        .route("/orders/:id", delete(api::cancel_order))
        // Swagger UI
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        // CORS layer
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
                .allow_headers(Any),
        )
        // Authentication middleware (applied to protected routes)
        .layer(axum::middleware::from_fn_with_state(
            app_state.clone(),
            middleware::auth_middleware,
        ))
        .with_state(app_state)
}
