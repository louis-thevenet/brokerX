pub mod handlers;
pub mod jwt;
pub mod templates;

use axum::{
    Router, middleware,
    routing::{get, post},
};
use domain::core::BrokerX;
use std::sync::{Arc, Mutex};
use tower_http::{services::ServeDir, trace::TraceLayer};

use handlers::{
    dashboard, deposit_page, deposit_submit, home, login_page, login_submit, logout,
    mfa_verify_page, mfa_verify_submit, orders_page, place_order_page, place_order_submit,
    register_page, register_submit, registration_verify_page, registration_verify_submit,
    resend_mfa,
};

// App state type - simplified to only contain BrokerX
pub type AppState = Arc<Mutex<BrokerX>>;

pub fn create_app(state: AppState) -> Router {
    // Public routes (no authentication required)
    let public_routes = Router::new()
        .route("/", get(home))
        .route("/login", get(login_page).post(login_submit))
        .route("/register", get(register_page).post(register_submit))
        .route("/verify-mfa", get(mfa_verify_page).post(mfa_verify_submit))
        .route(
            "/verify-registration",
            get(registration_verify_page).post(registration_verify_submit),
        )
        .route("/resend-mfa", get(resend_mfa));

    // Protected routes (authentication required)
    let protected_routes = Router::new()
        .route("/dashboard", get(dashboard))
        .route("/orders", get(orders_page))
        // add or remove money from account
        .route("/deposit", get(deposit_page).post(deposit_submit))
        .route(
            "/place_order",
            get(place_order_page).post(place_order_submit),
        )
        .route("/logout", post(logout))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            jwt::auth_middleware,
        ));

    Router::new()
        // Static file serving
        .nest_service("/static", ServeDir::new("static"))
        // Merge routes
        .merge(public_routes)
        .merge(protected_routes)
        // Add tracing middleware
        .layer(TraceLayer::new_for_http())
        // Add state
        .with_state(state)
}
