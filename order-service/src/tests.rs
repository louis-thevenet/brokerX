use axum::{
    Router,
    body::Body,
    http::{Method, Request, StatusCode},
};
use serde_json::Value;
use std::sync::Arc;
use tower::ServiceExt; // for `oneshot`
use uuid::Uuid;

use crate::models::{OrderSide, OrderType};
use crate::service::OrderService;
use crate::{ApiDoc, AppState};
use crate::{
    api::{CreateOrderRequest, OrderResponse},
    config::DatabaseConfig,
};
use tower_http::cors::CorsLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

// Create test setup that uses the order service database
async fn create_test_setup() -> (Router, Uuid, Uuid) {
    // Load environment variables for database connection
    dotenvy::dotenv().ok();

    // Use unique IDs for this test to avoid conflicts
    let test_user_id = Uuid::new_v4();
    let test_order_id = Uuid::new_v4();

    // Get the order database URL (ORDER_DATABASE_URL or fall back to DATABASE_URL)
    let order_db_url = std::env::var("ORDER_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .expect("Either ORDER_DATABASE_URL or DATABASE_URL must be set for tests");

    // Create unique test table name to avoid conflicts between test runs
    let test_table_name = format!("orders_test_{}", Uuid::new_v4().simple());

    // Create database config for order service
    let db_config = DatabaseConfig::from_url(order_db_url);

    // Create order service with test database configuration
    let order_service = Arc::new(
        OrderService::new_with_config(&test_table_name, &db_config)
            .await
            .expect("Failed to create order service for testing"),
    );

    let app_state = AppState { order_service };

    // Build the same router as in main.rs
    let app = Router::new()
        .route("/v1/health", axum::routing::get(crate::api::health_check))
        .route("/v1/orders", axum::routing::post(crate::api::create_order))
        .route("/v1/orders", axum::routing::get(crate::api::get_orders))
        .route("/v1/orders/{id}", axum::routing::get(crate::api::get_order))
        .route(
            "/v1/orders/{id}",
            axum::routing::put(crate::api::update_order),
        )
        .route(
            "/v1/orders/{id}",
            axum::routing::delete(crate::api::cancel_order),
        )
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .layer(CorsLayer::permissive())
        .with_state(app_state);

    (app, test_user_id, test_order_id)
}

#[tokio::test]
async fn test_health_check() {
    let (app, _, _) = create_test_setup().await;

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/v1/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_get_order_not_found() {
    let (app, _, _) = create_test_setup().await;
    let non_existent_id = Uuid::new_v4();

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/v1/orders/{non_existent_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_get_order_invalid_uuid() {
    let (app, _, _) = create_test_setup().await;

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/v1/orders/invalid-uuid")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_get_orders_for_user_empty() {
    let (app, user_id, _) = create_test_setup().await;

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/v1/orders?client_id={user_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let response_data: Value = serde_json::from_slice(&body).unwrap();
    let orders = response_data.get("data").unwrap().as_array().unwrap();
    assert!(orders.is_empty(), "New user should have no orders");
}

#[tokio::test]
async fn test_get_orders_for_nonexistent_user() {
    let (app, _, _) = create_test_setup().await;
    let non_existent_id = Uuid::new_v4();

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/v1/orders?client_id={non_existent_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let response_data: Value = serde_json::from_slice(&body).unwrap();
    let orders = response_data.get("data").unwrap().as_array().unwrap();
    assert!(orders.is_empty());
}

#[tokio::test]
async fn test_get_orders_for_user_invalid_uuid() {
    let (app, _, _) = create_test_setup().await;

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/v1/orders?client_id=invalid-uuid")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_create_order() {
    let (app, user_id, _) = create_test_setup().await;

    let create_request = CreateOrderRequest {
        client_id: user_id,
        symbol: "AAPL".to_string(),
        quantity: 10,
        order_side: OrderSide::Buy,
        order_type: OrderType::Market,
    };

    let request_body = serde_json::to_string(&create_request).unwrap();

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/v1/orders")
                .header("content-type", "application/json")
                .body(Body::from(request_body))
                .unwrap(),
        )
        .await
        .unwrap();

    // Order creation should succeed since we're a pure data service (no validation)
    // We expect CREATED (201) status
    assert_eq!(response.status(), StatusCode::CREATED);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let order_response: OrderResponse = serde_json::from_slice(&body).unwrap();

    // Verify the order was created with correct data
    assert_eq!(order_response.client_id, user_id);
    assert_eq!(order_response.symbol, "AAPL");
    assert_eq!(order_response.quantity, 10);
}

#[tokio::test]
async fn test_create_order_validation() {
    let (app, _, _) = create_test_setup().await;

    // Test with invalid data (missing required fields)
    let invalid_request = r#"{"symbol": "AAPL"}"#;

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/v1/orders")
                .header("content-type", "application/json")
                .body(Body::from(invalid_request))
                .unwrap(),
        )
        .await
        .unwrap();

    // The API might return 422 (Unprocessable Entity) for validation errors instead of 400
    assert!(
        response.status() == StatusCode::BAD_REQUEST
            || response.status() == StatusCode::UNPROCESSABLE_ENTITY
    );
}

#[tokio::test]
async fn test_create_limit_order() {
    let (app, user_id, _) = create_test_setup().await;

    let create_request = CreateOrderRequest {
        client_id: user_id,
        symbol: "GOOGL".to_string(),
        quantity: 5,
        order_side: OrderSide::Sell,
        order_type: OrderType::Limit { price: 150.0 },
    };

    let request_body = serde_json::to_string(&create_request).unwrap();

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/v1/orders")
                .header("content-type", "application/json")
                .body(Body::from(request_body))
                .unwrap(),
        )
        .await
        .unwrap();

    // Order creation should succeed since we're a pure data service (no validation)
    // We expect CREATED (201) status for limit orders too
    assert_eq!(response.status(), StatusCode::CREATED);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let order_response: OrderResponse = serde_json::from_slice(&body).unwrap();

    // Verify the order was created with correct data
    assert_eq!(order_response.client_id, user_id);
    assert_eq!(order_response.symbol, "GOOGL");
    assert_eq!(order_response.quantity, 5);
}

#[tokio::test]
async fn test_error_handling() {
    let (app, _, _) = create_test_setup().await;

    // Test various UUID formats for order retrieval
    let test_cases = vec![
        (
            "00000000-0000-0000-0000-000000000000",
            StatusCode::NOT_FOUND,
        ), // Valid UUID but not found
        ("not-a-uuid", StatusCode::BAD_REQUEST), // Invalid UUID
    ];

    for (uuid_str, expected_status) in test_cases {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri(format!("/v1/orders/{uuid_str}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), expected_status);
    }
}

#[tokio::test]
async fn test_swagger_ui() {
    let (app, _, _) = create_test_setup().await;

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/swagger-ui/")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Swagger UI should be available
    assert!(
        response.status() == StatusCode::OK
            || response.status() == StatusCode::MOVED_PERMANENTLY
            || response.status() == StatusCode::FOUND
    );
}

#[tokio::test]
async fn test_order_type_serialization() {
    // Test that our order types serialize/deserialize correctly
    let market_order = OrderType::Market;
    let limit_order = OrderType::Limit { price: 100.0 };

    let market_json = serde_json::to_string(&market_order).unwrap();
    let limit_json = serde_json::to_string(&limit_order).unwrap();

    // Market order serializes as tagged enum: {"type":"market"}
    assert!(market_json.contains("market"));
    // Limit order serializes as tagged enum with price: {"type":"limit","price":100.0}
    assert!(limit_json.contains("limit") && limit_json.contains("100"));

    // Test deserialization
    let deserialized_market: OrderType = serde_json::from_str(&market_json).unwrap();
    let deserialized_limit: OrderType = serde_json::from_str(&limit_json).unwrap();

    match deserialized_market {
        OrderType::Market => (),
        _ => panic!("Expected Market order type"),
    }

    match deserialized_limit {
        OrderType::Limit { price } => assert_eq!(price, 100.0),
        _ => panic!("Expected Limit order type"),
    }
}

#[tokio::test]
async fn test_order_side_serialization() {
    let buy = OrderSide::Buy;
    let sell = OrderSide::Sell;

    let buy_json = serde_json::to_string(&buy).unwrap();
    let sell_json = serde_json::to_string(&sell).unwrap();

    assert_eq!(buy_json, "\"buy\"");
    assert_eq!(sell_json, "\"sell\"");
}
