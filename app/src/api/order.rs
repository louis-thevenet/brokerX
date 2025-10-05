use axum::{Json, extract::State};
use domain::order::Order;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::AppState;

/// expose the Order OpenAPI to parent module
pub fn router(state: AppState) -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .with_state(state)
        .routes(routes!(get_order))
}

/// Get static order object
#[utoipa::path(get, path = "", responses((status = OK, body = Order)), tag = super::ORDER_TAG)]
async fn get_order(State(_state): State<AppState>) -> Json<Order> {
    Json(Order {
        client_id: todo!(),
        date: todo!(),
        symbol: todo!(),
        quantity: todo!(),
        status: todo!(),
        order_type: todo!(),
        order_side: todo!(),
    })
}
