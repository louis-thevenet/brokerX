use axum::Json;
use domain::user::User;
use serde::Serialize;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
/// expose the Customer OpenAPI to parent module
pub fn router() -> OpenApiRouter {
    OpenApiRouter::new().routes(routes!(get_customer))
}

/// Get customer
///
/// Just return a static Customer object
#[utoipa::path(get, path = "", responses((status = OK, body = User)), tag = super::USER_TAG)]
async fn get_customer() -> Json<User> {
    let email = String::from("test@test.com");
    let password = String::from("password");
    let firstname = String::from("John");
    let surname = String::from("Doe");
    let initial_balance = 1000.0;
    Json(User::new(email, password, firstname, surname, initial_balance).unwrap())
}
