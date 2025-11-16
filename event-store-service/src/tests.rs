use sqlx::PgPool;
use uuid::Uuid;

use crate::models::*;
use crate::repository::EventStoreRepository;

/// Create a test database pool and repository
/// Each test gets its own isolated setup
async fn create_test_setup() -> (PgPool, EventStoreRepository) {
    // Load environment variables
    dotenvy::dotenv().ok();

    // Get database URL from environment
    let db_url = std::env::var("EVENT_STORE_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .expect("Either EVENT_STORE_DATABASE_URL or DATABASE_URL must be set for tests");

    // Create connection pool
    let pool = PgPool::connect(&db_url)
        .await
        .expect("Failed to connect to test database");

    let repository = EventStoreRepository::new(pool.clone());

    (pool, repository)
}

/// Clean up test data
async fn cleanup_test_data(pool: &PgPool, order_id: OrderId) {
    // Delete in reverse order of dependencies
    let _ = sqlx::query("DELETE FROM outbox WHERE order_id = $1")
        .bind(order_id)
        .execute(pool)
        .await;

    let _ = sqlx::query("DELETE FROM saga_state WHERE order_id = $1")
        .bind(order_id)
        .execute(pool)
        .await;

    let _ = sqlx::query("DELETE FROM order_events WHERE order_id = $1")
        .bind(order_id)
        .execute(pool)
        .await;
}

#[tokio::test]
async fn test_save_event_with_outbox() {
    let (pool, repo) = create_test_setup().await;
    let order_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    let request = PublishEventRequest {
        order_id,
        user_id,
        event_type: OrderEventType::OrderCreated,
        symbol: "AAPL".to_string(),
        quantity: 100,
        order_side: OrderSide::Buy,
        order_type: OrderType::Limit { price: 150.0 },
        price: Some(150.0),
        filled_quantity: None,
        average_price: None,
        correlation_id: None,
        causation_id: None,
        metadata: None,
    };

    // Save event with outbox
    let result = repo.save_event_with_outbox(&request).await;
    assert!(result.is_ok());

    let event = result.unwrap();
    assert_eq!(event.order_id, order_id);
    assert_eq!(event.user_id, user_id);
    assert_eq!(event.symbol, "AAPL");
    assert_eq!(event.quantity, 100);
    assert!(event.sequence_number.is_some());

    // Verify event was saved
    let events = repo.get_order_events(order_id).await.unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_id, event.event_id);

    // Verify outbox message was created
    let outbox_messages = repo.get_unprocessed_outbox(10).await.unwrap();
    assert!(outbox_messages.iter().any(|m| m.order_id == order_id));

    // Cleanup
    cleanup_test_data(&pool, order_id).await;
}

#[tokio::test]
async fn test_get_order_events() {
    let (pool, repo) = create_test_setup().await;
    let order_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    // Create multiple events for the same order
    let events_to_create = vec![
        OrderEventType::OrderCreated,
        OrderEventType::OrderValidated,
        OrderEventType::OrderExecuted,
    ];

    for event_type in events_to_create {
        let request = PublishEventRequest {
            order_id,
            user_id,
            event_type: event_type.clone(),
            symbol: "TSLA".to_string(),
            quantity: 50,
            order_side: OrderSide::Sell,
            order_type: OrderType::Market,
            price: None,
            filled_quantity: if matches!(event_type, OrderEventType::OrderExecuted) {
                Some(50)
            } else {
                None
            },
            average_price: if matches!(event_type, OrderEventType::OrderExecuted) {
                Some(200.0)
            } else {
                None
            },
            correlation_id: None,
            causation_id: None,
            metadata: None,
        };

        repo.save_event_with_outbox(&request).await.unwrap();
    }

    // Get all events for the order
    let events = repo.get_order_events(order_id).await.unwrap();
    assert_eq!(events.len(), 3);

    // Verify they are in order
    assert!(matches!(events[0].event_type, OrderEventType::OrderCreated));
    assert!(matches!(
        events[1].event_type,
        OrderEventType::OrderValidated
    ));
    assert!(matches!(
        events[2].event_type,
        OrderEventType::OrderExecuted
    ));

    // Cleanup
    cleanup_test_data(&pool, order_id).await;
}

#[tokio::test]
async fn test_get_user_events() {
    let (pool, repo) = create_test_setup().await;
    let user_id = Uuid::new_v4();

    // Create events for multiple orders from the same user
    let order_ids = vec![Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4()];

    for order_id in &order_ids {
        let request = PublishEventRequest {
            order_id: *order_id,
            user_id,
            event_type: OrderEventType::OrderCreated,
            symbol: "GOOGL".to_string(),
            quantity: 10,
            order_side: OrderSide::Buy,
            order_type: OrderType::Market,
            price: None,
            filled_quantity: None,
            average_price: None,
            correlation_id: None,
            causation_id: None,
            metadata: None,
        };

        repo.save_event_with_outbox(&request).await.unwrap();
    }

    // Get user events
    let events = repo.get_user_events(user_id, 10, 0).await.unwrap();
    assert_eq!(events.len(), 3);

    // Test pagination
    let page1 = repo.get_user_events(user_id, 2, 0).await.unwrap();
    assert_eq!(page1.len(), 2);

    let page2 = repo.get_user_events(user_id, 2, 2).await.unwrap();
    assert_eq!(page2.len(), 1);

    // Cleanup
    for order_id in order_ids {
        cleanup_test_data(&pool, order_id).await;
    }
}

#[tokio::test]
async fn test_saga_state_lifecycle() {
    let (pool, repo) = create_test_setup().await;
    let order_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let correlation_id = Uuid::new_v4();

    // Initialize saga state
    let result = repo
        .init_saga_state(
            order_id,
            user_id,
            "MSFT",
            100,
            &OrderSide::Buy,
            &OrderType::Limit { price: 300.0 },
            Some(300.0),
            correlation_id,
        )
        .await;
    assert!(result.is_ok());

    // Get saga state
    let saga = repo.get_saga_state(order_id).await.unwrap();
    assert!(saga.is_some());

    let saga = saga.unwrap();
    assert_eq!(saga.order_id, order_id);
    assert_eq!(saga.user_id, user_id);
    assert_eq!(saga.current_step, SagaStep::Created);
    assert_eq!(saga.symbol, "MSFT");
    assert_eq!(saga.quantity, 100);
    assert!(!saga.compensation_needed);

    // Update saga state to Validating
    repo.update_saga_state(order_id, SagaStep::Validating, None, None, None, false)
        .await
        .unwrap();

    let saga = repo.get_saga_state(order_id).await.unwrap().unwrap();
    assert_eq!(saga.current_step, SagaStep::Validating);

    // Update saga state to Executed with fill info
    repo.update_saga_state(
        order_id,
        SagaStep::Executed,
        Some(100),
        Some(299.5),
        None,
        false,
    )
    .await
    .unwrap();

    let saga = repo.get_saga_state(order_id).await.unwrap().unwrap();
    assert_eq!(saga.current_step, SagaStep::Executed);
    assert_eq!(saga.filled_quantity, Some(100));
    assert_eq!(saga.average_price, Some(299.5));

    // Complete saga
    repo.update_saga_state(order_id, SagaStep::Completed, None, None, None, false)
        .await
        .unwrap();

    let saga = repo.get_saga_state(order_id).await.unwrap().unwrap();
    assert_eq!(saga.current_step, SagaStep::Completed);
    assert!(saga.completed_at.is_some());

    // Cleanup
    cleanup_test_data(&pool, order_id).await;
}

#[tokio::test]
async fn test_saga_state_compensation() {
    let (pool, repo) = create_test_setup().await;
    let order_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let correlation_id = Uuid::new_v4();

    // Initialize saga
    repo.init_saga_state(
        order_id,
        user_id,
        "AMZN",
        50,
        &OrderSide::Sell,
        &OrderType::Market,
        None,
        correlation_id,
    )
    .await
    .unwrap();

    // Move to Failed with compensation needed
    repo.update_saga_state(
        order_id,
        SagaStep::Failed,
        None,
        None,
        Some("Insufficient funds".to_string()),
        true,
    )
    .await
    .unwrap();

    let saga = repo.get_saga_state(order_id).await.unwrap().unwrap();
    assert_eq!(saga.current_step, SagaStep::Failed);
    assert!(saga.compensation_needed);
    assert_eq!(saga.error_message, Some("Insufficient funds".to_string()));

    // Move to Compensating
    repo.update_saga_state(order_id, SagaStep::Compensating, None, None, None, true)
        .await
        .unwrap();

    // Complete compensation
    repo.update_saga_state(order_id, SagaStep::Compensated, None, None, None, false)
        .await
        .unwrap();

    let saga = repo.get_saga_state(order_id).await.unwrap().unwrap();
    assert_eq!(saga.current_step, SagaStep::Compensated);
    assert!(saga.completed_at.is_some());

    // Cleanup
    cleanup_test_data(&pool, order_id).await;
}

#[tokio::test]
async fn test_outbox_processing() {
    let (pool, repo) = create_test_setup().await;
    let order_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    // Create an event (which creates outbox message)
    let request = PublishEventRequest {
        order_id,
        user_id,
        event_type: OrderEventType::OrderCreated,
        symbol: "NVDA".to_string(),
        quantity: 25,
        order_side: OrderSide::Buy,
        order_type: OrderType::Market,
        price: None,
        filled_quantity: None,
        average_price: None,
        correlation_id: None,
        causation_id: None,
        metadata: None,
    };

    repo.save_event_with_outbox(&request).await.unwrap();

    // Get unprocessed outbox messages
    let messages = repo.get_unprocessed_outbox(10).await.unwrap();
    assert!(messages.iter().any(|m| m.order_id == order_id));

    let message = messages.iter().find(|m| m.order_id == order_id).unwrap();
    assert_eq!(message.processing_attempts, 0);
    assert!(message.processed_at.is_none());

    // Mark as processed
    repo.mark_outbox_processed(message.outbox_id).await.unwrap();

    // Verify it's no longer in unprocessed
    let messages = repo.get_unprocessed_outbox(10).await.unwrap();
    assert!(!messages.iter().any(|m| m.outbox_id == message.outbox_id));

    // Cleanup
    cleanup_test_data(&pool, order_id).await;
}

#[tokio::test]
async fn test_outbox_failure_handling() {
    let (pool, repo) = create_test_setup().await;
    let order_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    // Create an event
    let request = PublishEventRequest {
        order_id,
        user_id,
        event_type: OrderEventType::OrderCreated,
        symbol: "META".to_string(),
        quantity: 15,
        order_side: OrderSide::Sell,
        order_type: OrderType::Limit { price: 350.0 },
        price: Some(350.0),
        filled_quantity: None,
        average_price: None,
        correlation_id: None,
        causation_id: None,
        metadata: None,
    };

    repo.save_event_with_outbox(&request).await.unwrap();

    // Get the message
    let messages = repo.get_unprocessed_outbox(10).await.unwrap();
    let message = messages.iter().find(|m| m.order_id == order_id).unwrap();

    // Mark as failed multiple times
    for i in 1..=3 {
        repo.mark_outbox_failed(message.outbox_id, &format!("Error attempt {i}"))
            .await
            .unwrap();
    }

    // Get the message again and verify attempts
    let messages = repo.get_unprocessed_outbox(10).await.unwrap();
    let updated_message = messages.iter().find(|m| m.order_id == order_id).unwrap();

    assert_eq!(updated_message.processing_attempts, 3);
    assert!(updated_message.last_error.is_some());
    assert!(
        updated_message
            .last_error
            .as_ref()
            .unwrap()
            .contains("Error attempt 3")
    );

    // Cleanup
    cleanup_test_data(&pool, order_id).await;
}

#[tokio::test]
async fn test_transactional_outbox_atomicity() {
    let (pool, repo) = create_test_setup().await;
    let order_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    let request = PublishEventRequest {
        order_id,
        user_id,
        event_type: OrderEventType::OrderCreated,
        symbol: "NFLX".to_string(),
        quantity: 30,
        order_side: OrderSide::Buy,
        order_type: OrderType::Market,
        price: None,
        filled_quantity: None,
        average_price: None,
        correlation_id: None,
        causation_id: None,
        metadata: None,
    };

    // Save event with outbox
    repo.save_event_with_outbox(&request).await.unwrap();

    // Verify both event and outbox were created atomically
    let events = repo.get_order_events(order_id).await.unwrap();
    let outbox = repo.get_unprocessed_outbox(100).await.unwrap();

    assert_eq!(events.len(), 1);
    assert!(outbox.iter().any(|m| m.order_id == order_id));

    // Verify the outbox payload contains the event data
    let outbox_message = outbox.iter().find(|m| m.order_id == order_id).unwrap();
    let payload = &outbox_message.payload;

    assert_eq!(payload["order_id"], order_id.to_string());
    assert_eq!(payload["symbol"], "NFLX");
    assert_eq!(payload["quantity"], 30);

    // Cleanup
    cleanup_test_data(&pool, order_id).await;
}

#[tokio::test]
async fn test_correlation_and_causation_ids() {
    let (pool, repo) = create_test_setup().await;
    let order_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let correlation_id = Uuid::new_v4();
    let causation_id = Uuid::new_v4();

    let request = PublishEventRequest {
        order_id,
        user_id,
        event_type: OrderEventType::OrderCreated,
        symbol: "TSLA".to_string(),
        quantity: 20,
        order_side: OrderSide::Buy,
        order_type: OrderType::Market,
        price: None,
        filled_quantity: None,
        average_price: None,
        correlation_id: Some(correlation_id),
        causation_id: Some(causation_id),
        metadata: None,
    };

    repo.save_event_with_outbox(&request).await.unwrap();

    let events = repo.get_order_events(order_id).await.unwrap();
    assert_eq!(events.len(), 1);

    let event = &events[0];
    assert_eq!(event.correlation_id, correlation_id);
    assert_eq!(event.causation_id, Some(causation_id));

    // Cleanup
    cleanup_test_data(&pool, order_id).await;
}
