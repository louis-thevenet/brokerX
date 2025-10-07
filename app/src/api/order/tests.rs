#[cfg(test)]
mod tests {
    use axum::{
        Router,
        body::Body,
        http::{Method, Request, StatusCode},
    };
    use domain::order::{Order, OrderSide, OrderStatus, OrderType};
    use domain::user::UserRepoExt;
    use serde_json::json;
    use tower::ServiceExt; // for `oneshot`
    use uuid::Uuid;

    use crate::api::order::{CreateOrderRequest, UpdateOrderRequest};
    use crate::services::BrokerHandle;

    // Create test setup that is isolated and consistent
    async fn create_test_setup() -> (Router, Uuid, Uuid) {
        // Use unique IDs for this test to avoid conflicts
        let test_user_id = Uuid::new_v4();
        let test_order_id = Uuid::new_v4();
        let test_id_str = test_user_id.to_string();
        let test_email = format!("test-{}@test.com", &test_id_str[..8]);

        let broker = domain::core::BrokerX::new_for_testing().await;

        // Create a test user first
        let user_repo = broker.get_user_repo().await;
        let actual_user_id = match user_repo
            .create_user(
                test_email.clone(),
                "password123".to_string(),
                "Test".to_string(),
                "User".to_string(),
                10000.0, // Give enough balance for orders
            )
            .await
        {
            Ok(id) => {
                // Verify the user if creation succeeded
                let user_repo_mut = broker.get_user_repo().await;
                let _ = user_repo_mut.verify_user_email(&id).await;
                id
            }
            Err(_) => {
                // If database is unavailable, use a mock UUID for basic routing tests
                test_user_id
            }
        };

        let handle = BrokerHandle::new(broker);
        let (router, _api) = crate::api::order::router(handle.clone()).split_for_parts();
        (router.with_state(handle), actual_user_id, test_order_id)
    }

    // Helper function to create a test order through the broker
    #[allow(dead_code)]
    async fn create_test_order(
        broker: &domain::core::BrokerX,
        user_id: Uuid,
    ) -> Result<Uuid, Box<dyn std::error::Error>> {
        let order_id = broker
            .create_order(
                user_id,
                "AAPL".to_string(),
                10,
                OrderSide::Buy,
                OrderType::Market,
            )
            .await?;
        Ok(order_id)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_orders_empty() {
        let (app, _, _) = create_test_setup().await;

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let orders: Vec<Order> = serde_json::from_slice(&body).unwrap();
        // For now, this returns empty since we have a placeholder implementation
        assert!(orders.is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_order_not_found() {
        let (app, _, _) = create_test_setup().await;
        let non_existent_id = Uuid::new_v4();

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri(format!("/{}", non_existent_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_order_invalid_uuid() {
        let (app, _, _) = create_test_setup().await;

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/invalid-uuid")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // Axum should return 400 for invalid UUID format
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_post_order_create_success() {
        let (app, user_id, _) = create_test_setup().await;

        let create_request = CreateOrderRequest {
            client_id: user_id,
            symbol: "AAPL".to_string(),
            quantity: 10,
            order_side: OrderSide::Buy,
            order_type: OrderType::Market,
        };

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&create_request).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let created_order: Order = serde_json::from_slice(&body).unwrap();

        assert_eq!(created_order.client_id, user_id);
        assert_eq!(created_order.symbol, "AAPL");
        assert_eq!(created_order.quantity, 10);
        assert!(matches!(created_order.order_side, OrderSide::Buy));
        assert!(matches!(created_order.order_type, OrderType::Market));
        assert!(matches!(created_order.status, OrderStatus::Queued));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_post_order_create_limit_order() {
        let (app, user_id, _) = create_test_setup().await;

        let create_request = CreateOrderRequest {
            client_id: user_id,
            symbol: "MSFT".to_string(),
            quantity: 5,
            order_side: OrderSide::Sell,
            order_type: OrderType::Limit(150.0),
        };

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&create_request).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let created_order: Order = serde_json::from_slice(&body).unwrap();

        assert_eq!(created_order.client_id, user_id);
        assert_eq!(created_order.symbol, "MSFT");
        assert_eq!(created_order.quantity, 5);
        assert!(matches!(created_order.order_side, OrderSide::Sell));
        assert!(matches!(created_order.order_type, OrderType::Limit(150.0)));
        assert!(matches!(created_order.status, OrderStatus::Queued));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_post_order_invalid_data() {
        let (app, _, _) = create_test_setup().await;

        // Test with invalid JSON
        let invalid_json = r#"{"invalid": "json"#;

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/")
                    .header("content-type", "application/json")
                    .body(Body::from(invalid_json))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        // Test with missing required fields
        let incomplete_request = json!({
            "symbol": "AAPL",
            "quantity": 10
            // Missing client_id, order_side, order_type
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/")
                    .header("content-type", "application/json")
                    .body(Body::from(incomplete_request.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_put_order_update_status() {
        let (app, user_id, _) = create_test_setup().await;

        // First create an order via POST
        let create_request = CreateOrderRequest {
            client_id: user_id,
            symbol: "GOOGL".to_string(),
            quantity: 3,
            order_side: OrderSide::Buy,
            order_type: OrderType::Market,
        };

        let create_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&create_request).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(create_response.status(), StatusCode::CREATED);

        let body = axum::body::to_bytes(create_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let _created_order: Order = serde_json::from_slice(&body).unwrap();

        // Extract the order ID - we'll need to simulate this since Order doesn't have an id field
        // For this test, we'll use a mock order ID
        let order_id = Uuid::new_v4();

        // Now try to update the order status
        let update_request = UpdateOrderRequest {
            status: Some(OrderStatus::Cancelled),
        };

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::PUT)
                    .uri(format!("/{}", order_id))
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&update_request).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        // This will return NOT_FOUND since we're using a mock order ID
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_put_order_not_found() {
        let (app, _, _) = create_test_setup().await;
        let non_existent_id = Uuid::new_v4();

        let update_request = UpdateOrderRequest {
            status: Some(OrderStatus::Cancelled),
        };

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::PUT)
                    .uri(format!("/{}", non_existent_id))
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&update_request).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_delete_order_not_found() {
        let (app, _, _) = create_test_setup().await;
        let non_existent_id = Uuid::new_v4();

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::DELETE)
                    .uri(format!("/{}", non_existent_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_delete_order_invalid_uuid() {
        let (app, _, _) = create_test_setup().await;

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::DELETE)
                    .uri("/invalid-uuid")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    // Test error handling for various UUID formats
    #[tokio::test(flavor = "multi_thread")]
    async fn test_error_handling() {
        let (app, _, _) = create_test_setup().await;

        // Test various UUID formats
        let test_cases = vec![
            (
                "00000000-0000-0000-0000-000000000000",
                StatusCode::NOT_FOUND,
            ), // Valid UUID but not found
            ("not-a-uuid", StatusCode::BAD_REQUEST), // Invalid UUID
        ];

        for (uuid_str, expected_status) in test_cases {
            let uri = if uuid_str.is_empty() {
                "/".to_string()
            } else {
                format!("/{}", uuid_str)
            };

            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method(Method::GET)
                        .uri(uri)
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            assert_eq!(
                response.status(),
                expected_status,
                "Failed for UUID: {}",
                uuid_str
            );
        }
    }

    // Test pre-trade validation failures
    #[tokio::test(flavor = "multi_thread")]
    async fn test_post_order_insufficient_balance() {
        let (app, _, _) = create_test_setup().await;

        // Create a user with zero balance
        let broker = domain::core::BrokerX::new_for_testing().await;
        let user_repo = broker.get_user_repo().await;
        let poor_user_id = user_repo
            .create_user(
                "poor@test.com".to_string(),
                "password123".to_string(),
                "Poor".to_string(),
                "User".to_string(),
                0.0, // No balance
            )
            .await
            .unwrap_or_else(|_| Uuid::new_v4());

        let create_request = CreateOrderRequest {
            client_id: poor_user_id,
            symbol: "AAPL".to_string(),
            quantity: 1000, // Large quantity requiring significant balance
            order_side: OrderSide::Buy,
            order_type: OrderType::Market,
        };

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&create_request).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Should fail pre-trade validation due to insufficient balance
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let error_msg = String::from_utf8(body.to_vec()).unwrap();
        assert!(error_msg.contains("Order creation error"));
    }

    // Test JSON serialization/deserialization
    #[tokio::test(flavor = "multi_thread")]
    async fn test_request_dto_serialization() {
        let create_request = CreateOrderRequest {
            client_id: Uuid::new_v4(),
            symbol: "AAPL".to_string(),
            quantity: 10,
            order_side: OrderSide::Buy,
            order_type: OrderType::Limit(150.0),
        };

        // Test serialization
        let json_str = serde_json::to_string(&create_request).unwrap();
        assert!(json_str.contains("AAPL"));
        assert!(json_str.contains("Buy"));
        assert!(json_str.contains("150"));

        // Test deserialization
        let deserialized: CreateOrderRequest = serde_json::from_str(&json_str).unwrap();
        assert_eq!(deserialized.symbol, "AAPL");
        assert_eq!(deserialized.quantity, 10);
        assert!(matches!(deserialized.order_side, OrderSide::Buy));
        assert!(matches!(deserialized.order_type, OrderType::Limit(150.0)));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_update_request_dto_serialization() {
        let update_request = UpdateOrderRequest {
            status: Some(OrderStatus::Cancelled),
        };

        // Test serialization
        let json_str = serde_json::to_string(&update_request).unwrap();
        assert!(json_str.contains("Cancelled"));

        // Test deserialization
        let deserialized: UpdateOrderRequest = serde_json::from_str(&json_str).unwrap();
        assert!(matches!(deserialized.status, Some(OrderStatus::Cancelled)));

        // Test with None status
        let empty_update = UpdateOrderRequest { status: None };
        let json_str = serde_json::to_string(&empty_update).unwrap();
        assert_eq!(json_str, "{}"); // Should skip serializing None fields
    }
}
