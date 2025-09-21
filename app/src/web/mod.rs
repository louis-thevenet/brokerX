pub mod auth;
pub mod handlers;
pub mod templates;

use std::sync::{Arc, Mutex};
use axum::{
    Router,
    routing::{get, post},
};
use tower_http::{services::ServeDir, trace::TraceLayer};
use domain::core::BrokerX;

use handlers::{home, login_page, login_submit, logout, register_page, register_submit};

// App state type
pub type AppState = Arc<Mutex<BrokerX>>;

pub fn create_app(state: AppState) -> Router {
    Router::new()
        // Static file serving
        .nest_service("/static", ServeDir::new("static"))
        // Authentication routes
        .route("/", get(home))
        .route("/login", get(login_page).post(login_submit))
        .route("/register", get(register_page).post(register_submit))
        .route("/logout", post(logout))
        // Add tracing middleware
        .layer(TraceLayer::new_for_http())
        // Add state
        .with_state(state)
}
