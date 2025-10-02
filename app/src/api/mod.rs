use std::{
    net::{Ipv4Addr, TcpListener},
    sync::{Arc, Mutex},
};

use axum::Router;
use domain::core::BrokerX;
use utoipa::{OpenApi, openapi};
use utoipa_axum::{
    router::{self, OpenApiRouter},
    routes,
};
use utoipa_swagger_ui::SwaggerUi;
mod order;
mod user;

const USER_TAG: &str = "user";
const ORDER_TAG: &str = "order";

#[derive(OpenApi)]
#[openapi(
    tags(
        (name = USER_TAG, description = "Customer API endpoints"),
        (name = ORDER_TAG, description = "Order API endpoints")
    )
)]
struct ApiDoc;

/// Get health of the API.
#[utoipa::path(
    method(get, head),
    path = "/api/health",
    responses(
        (status = OK, description = "Success", body = str, content_type = "text/plain")
    )
)]
async fn health() -> &'static str {
    "ok"
}

pub type AppState = Arc<Mutex<BrokerX>>;

pub fn create_api(state: AppState) -> Router {
    let (router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .routes(routes!(health))
        .nest("/api/user", user::router())
        .nest("/api/order", order::router())
        .split_for_parts();

    router.merge(SwaggerUi::new("/swagger-ui").url("/apidoc/openapi.json", api))
}
