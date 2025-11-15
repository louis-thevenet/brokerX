use serde::{Deserialize, Serialize};

/// Shared event models for event-driven architecture
/// These models are used across all microservices

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderEventType {
    OrderCreated,
    OrderValidated,
    OrderRejected,
    OrderExecuted,
    OrderPartiallyFilled,
    OrderFilled,
    OrderCancelled,
    FundsDebited,
    HoldingsUpdated,
    OrderCompleted,
    OrderFailed,
    // Compensation events
    RefundIssued,
    HoldingsReverted,
    OrderCompensated,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum OrderSide {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum OrderType {
    Market,
    Limit { price: f64 },
}

/// Event payload published to Redis Pub/Sub
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderEventPayload {
    pub event_id: String,
    pub order_id: String,
    pub user_id: String,
    pub event_type: OrderEventType,
    pub symbol: String,
    pub quantity: i64,
    pub order_side: OrderSide,
    pub order_type: OrderType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filled_quantity: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub average_price: Option<f64>,
    pub correlation_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub causation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
    pub occurred_at: String, // ISO 8601 timestamp
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_order_event_payload() {
        let payload = OrderEventPayload {
            event_id: "evt_123".to_string(),
            order_id: "ord_456".to_string(),
            user_id: "usr_789".to_string(),
            event_type: OrderEventType::OrderCreated,
            symbol: "AAPL".to_string(),
            quantity: 100,
            order_side: OrderSide::Buy,
            order_type: OrderType::Market,
            price: None,
            filled_quantity: None,
            average_price: None,
            correlation_id: "corr_123".to_string(),
            causation_id: None,
            metadata: None,
            occurred_at: "2025-11-15T12:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("ORDER_CREATED"));
        assert!(json.contains("AAPL"));
    }

    #[test]
    fn test_deserialize_order_event_payload() {
        let json = r#"{
            "event_id": "evt_123",
            "order_id": "ord_456",
            "user_id": "usr_789",
            "event_type": "ORDER_CREATED",
            "symbol": "AAPL",
            "quantity": 100,
            "order_side": "buy",
            "order_type": {"type": "market"},
            "correlation_id": "corr_123",
            "occurred_at": "2025-11-15T12:00:00Z"
        }"#;

        let payload: OrderEventPayload = serde_json::from_str(json).unwrap();
        assert_eq!(payload.event_type, OrderEventType::OrderCreated);
        assert_eq!(payload.symbol, "AAPL");
        assert_eq!(payload.quantity, 100);
    }
}
