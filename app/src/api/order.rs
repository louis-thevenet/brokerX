use axum::Json;
use domain::order::Order;
use domain::user::User;
use serde::Serialize;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

/// expose the Order OpenAPI to parent module
pub fn router() -> OpenApiRouter {
    OpenApiRouter::new().routes(routes!(get_order))
}

/// Get static order object
#[utoipa::path(get, path = "", responses((status = OK, body = Order)), tag = super::ORDER_TAG)]
async fn get_order() -> Json<Order> {
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
