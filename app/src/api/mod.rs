use axum::Router;
use utoipa::OpenApi;
use utoipa_axum::{router::OpenApiRouter, routes};
use utoipa_swagger_ui::SwaggerUi;

use crate::services::BrokerHandle;

mod order;
mod user;

const USER_TAG: &str = "user";
const ORDER_TAG: &str = "order";

#[derive(OpenApi)]
#[openapi(
    paths(
        health,
    ),
    components(
        schemas(
            order::CreateOrderRequest,
            order::UpdateOrderRequest,
            user::UpdateUserRequest
        )
    ),
    tags(
        (name = USER_TAG, description = "User API endpoints"),
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

pub type AppState = BrokerHandle;

pub fn create_api(state: AppState) -> Router {
    let (router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .routes(routes!(health))
        .nest("/api/user", user::router(state.clone()))
        .nest("/api/order", order::router(state.clone()))
        .split_for_parts();

    router
        .merge(SwaggerUi::new("/swagger-ui").url("/apidoc/openapi.json", api))
        .with_state(state)
}
