mod api;
mod config;
mod models;
mod repository;
mod service;

#[cfg(test)]
mod tests;

use axum::{
    Router,
    routing::{get, post, put},
};
use database_adapter::config::DatabaseConfig;
use service::UserService;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

#[derive(Clone)]
pub struct AppState {
    user_service: Arc<UserService>,
}

#[derive(OpenApi)]
#[openapi(
    paths(
        api::get_user,
        api::create_user,
        api::update_user,
        api::get_user_by_email,
        api::verify_user_email,
        api::authenticate_user,
        api::health_check,
    ),
    components(schemas(
        models::User,
        models::Holding,
        api::CreateUserRequest,
        api::UpdateUserRequest,
        api::AuthenticateRequest,
        api::HealthResponse,
    )),
    tags(
        (name = "users", description = "User management operations"),
        (name = "auth", description = "Authentication operations"),
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

    // Use USER_DATABASE_URL or fall back to DATABASE_URL for user service
    let user_db_url = std::env::var("USER_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .expect("Either USER_DATABASE_URL or DATABASE_URL must be set");
    let db_config = DatabaseConfig::from_url(user_db_url);

    info!("Starting User Service on port {}", config.port);

    // Initialize user service with dedicated database
    let user_service = Arc::new(
        UserService::new_with_config("users", &db_config)
            .await
            .expect("Failed to initialize user service"),
    );

    let app_state = AppState { user_service };

    // Build routes
    let app = Router::new()
        .route("/health", get(api::health_check))
        .route("/users", post(api::create_user))
        .route("/users/{id}", get(api::get_user))
        .route("/users/{id}", put(api::update_user))
        .route("/users/by-email/{email}", get(api::get_user_by_email))
        .route("/users/{id}/verify", post(api::verify_user_email))
        .route("/auth/login", post(api::authenticate_user))
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .layer(CorsLayer::permissive())
        .with_state(app_state);

    // Start server
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.port)).await?;
    info!("User Service listening on {}", listener.local_addr()?);

    axum::serve(listener, app).await?;

    Ok(())
}
