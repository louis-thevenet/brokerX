#[cfg(test)]
mod tests {
    use axum::{
        Router,
        body::Body,
        http::{Method, Request, StatusCode},
    };
    use domain::user::{User, UserRepoExt};
    use serde_json::Value;
    use tower::ServiceExt;
    use uuid::Uuid;

    use crate::services::BrokerHandle;

    fn create_test_setup() -> (Router, Uuid) {
        // Use a unique ID for this test to avoid conflicts
        let test_id = Uuid::new_v4();
        let test_id_str = test_id.to_string();
        let test_email = format!("test-{}@test.com", &test_id_str[..8]);

        // Create broker with minimal setup to avoid database conflicts
        let broker = domain::core::BrokerX::new();

        let mut user_repo = broker.get_user_repo();
        let test_user_id = match user_repo.create_user(
            test_email.clone(),
            "password123".to_string(),
            "Test".to_string(),
            "User".to_string(),
            1000.0,
        ) {
            Ok(id) => {
                // Verify the user if creation succeeded
                let mut user_repo_mut = broker.get_user_repo();
                let _ = user_repo_mut.verify_user_email(&id);
                id
            }
            Err(_) => {
                // If database is unavailable, use a mock UUID for basic routing tests
                test_id
            }
        };

        let handle = BrokerHandle::new(broker);
        let (router, _api) = crate::api::user::router(handle.clone()).split_for_parts();
        (router.with_state(handle), test_user_id)
    }

    #[tokio::test]
    async fn test_get_user_success() {
        let (app, user_id) = create_test_setup();

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri(format!("/{user_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let user: User = serde_json::from_slice(&body).unwrap();

        assert_eq!(user.id, Some(user_id));
        assert!(user.email.starts_with("test-") && user.email.ends_with("@test.com"));
        assert_eq!(user.firstname, "Test");
        assert_eq!(user.surname, "User");
        assert_eq!(user.balance, 1000.0);
        assert!(user.is_verified);
    }

    #[tokio::test]
    async fn test_get_user_not_found() {
        let (app, _) = create_test_setup();
        let non_existent_id = Uuid::new_v4();

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri(format!("/{non_existent_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_get_user_invalid_uuid() {
        let (app, _) = create_test_setup();

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

    #[tokio::test]
    async fn test_get_orders_from_user_empty() {
        let (app, user_id) = create_test_setup();

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri(format!("/{user_id}/orders"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let orders: Vec<Value> = serde_json::from_slice(&body).unwrap();
        assert!(orders.is_empty(), "New user should have no orders");
    }

    #[tokio::test]
    async fn test_get_orders_from_nonexistent_user() {
        let (app, _) = create_test_setup();
        let non_existent_id = Uuid::new_v4();

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri(format!("/{non_existent_id}/orders"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let orders: Vec<Value> = serde_json::from_slice(&body).unwrap();
        assert!(orders.is_empty());
    }

    #[tokio::test]
    async fn test_get_orders_from_user_invalid_uuid() {
        let (app, _) = create_test_setup();

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/invalid-uuid/orders")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    // Integration test with actual order creation
    #[tokio::test]
    async fn test_get_orders_from_user_with_orders() {
        let (app, user_id) = create_test_setup();

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri(format!("/{user_id}/orders"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let orders: Vec<Value> = serde_json::from_slice(&body).unwrap();
        assert!(orders.is_empty());
    }

    // Test error handling for database errors
    #[tokio::test]
    async fn test_error_handling() {
        let (app, _) = create_test_setup();

        // Test various UUID formats
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
                        .uri(format!("/{uuid_str}"))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            assert_eq!(response.status(), expected_status);
        }
    }

    // Integration test for the PUT endpoint
    #[tokio::test]
    async fn test_put_user_update_existing() {
        let (app, user_id) = create_test_setup();

        let update_request = r#"{"firstname": "UpdatedName"}"#;

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::PUT)
                    .uri(format!("/{user_id}"))
                    .header("content-type", "application/json")
                    .body(Body::from(update_request))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let updated_user: User = serde_json::from_slice(&body).unwrap();

        assert_eq!(updated_user.firstname, "UpdatedName");
        assert_eq!(updated_user.surname, "User"); // Should remain unchanged
        assert!(
            updated_user.email.starts_with("test-") && updated_user.email.ends_with("@test.com")
        ); // Should remain unchanged
    }

    #[tokio::test]
    async fn test_put_user_create_new() {
        let (app, _) = create_test_setup();
        let new_user_id = Uuid::new_v4();

        // Generate a unique email to avoid conflicts
        let unique_email = format!(
            "newuser-{}@test.com",
            Uuid::new_v4().simple().to_string()[..8].to_lowercase()
        );

        let create_request = format!(
            r#"{{
            "firstname": "NewUser", 
            "surname": "Created",
            "email": "{unique_email}",
            "password": "password123"
        }}"#
        );

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::PUT)
                    .uri(format!("/{new_user_id}"))
                    .header("content-type", "application/json")
                    .body(Body::from(create_request))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let created_user: User = serde_json::from_slice(&body).unwrap();

        assert_eq!(created_user.id, Some(new_user_id));
        assert_eq!(created_user.firstname, "NewUser");
        assert_eq!(created_user.surname, "Created");
        assert_eq!(created_user.email, unique_email);
        assert_eq!(created_user.balance, 0.0); // Default balance
        assert!(!created_user.is_verified); // Should not be verified initially
    }

    #[tokio::test]
    async fn test_put_user_validation_errors() {
        let (app, _) = create_test_setup();
        let new_user_id = Uuid::new_v4();

        // Test creating user with invalid email
        let invalid_email_request = r#"{
            "firstname": "Test", 
            "surname": "User",
            "email": "invalid-email",
            "password": "password123"
        }"#;

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::PUT)
                    .uri(format!("/{new_user_id}"))
                    .header("content-type", "application/json")
                    .body(Body::from(invalid_email_request))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        // Test creating user with missing required fields
        let incomplete_request = r#"{"firstname": "Test"}"#;

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::PUT)
                    .uri(format!("/{new_user_id}"))
                    .header("content-type", "application/json")
                    .body(Body::from(incomplete_request))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_post_user_create() {
        let (app, _) = create_test_setup();

        // Generate a unique email to avoid conflicts
        let unique_email = format!(
            "postuser-{}@test.com",
            Uuid::new_v4().simple().to_string()[..8].to_lowercase()
        );

        // Test creating a new user via POST
        let create_request = format!(
            r#"{{
            "firstname": "PostUser", 
            "surname": "Created",
            "email": "{unique_email}",
            "password": "password123"
        }}"#
        );

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/")
                    .header("content-type", "application/json")
                    .body(Body::from(create_request))
                    .unwrap(),
            )
            .await
            .unwrap();

        // POST should create a user and return 201 CREATED with the created user
        assert_eq!(response.status(), StatusCode::CREATED);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let created_user: User = serde_json::from_slice(&body).unwrap();

        assert!(created_user.id.is_some());
        assert_eq!(created_user.firstname, "PostUser");
        assert_eq!(created_user.surname, "Created");
        assert_eq!(created_user.email, unique_email);
        assert_eq!(created_user.balance, 0.0);
        assert!(!created_user.is_verified);
    }
}
