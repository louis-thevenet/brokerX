use askama::Template;
use domain::order::{Order, OrderId, OrderSide, OrderStatus, OrderType};

#[derive(Template)]
#[template(path = "login.html")]
pub struct LoginTemplate {
    pub error: Option<String>,
}

#[derive(Template)]
#[template(path = "register.html")]
pub struct RegisterTemplate {
    pub error: Option<String>,
}

#[derive(Template)]
#[template(path = "mfa_verify.html")]
pub struct MfaVerifyTemplate {
    pub challenge_id: String,
    pub error: Option<String>,
}

#[derive(Template)]
#[template(path = "registration_verify.html")]
pub struct RegistrationVerifyTemplate {
    pub challenge_id: String,
    pub user_id: String,
    pub error: Option<String>,
}

#[derive(Template)]
#[template(path = "dashboard.html")]
pub struct DashboardTemplate<'a> {
    pub username: &'a str,
    pub firstname: &'a str,
    pub surname: &'a str,
    pub email: &'a str,
    pub account_balance: f64,
    pub recent_orders: Vec<OrderDisplayData>,
}

// Struct for order display in templates
#[derive(Clone)]
pub struct OrderDisplayData {
    pub id: String,
    pub symbol: String,
    pub quantity: u64,
    pub price: f64,
    pub order_type: String, // "Buy" or "Sell"
    pub order_kind: String, // "Market" or "Limit"
    pub status: String,
    pub date: String,
    pub total: f64,
    pub status_tooltip: Option<String>, // Additional status information for tooltips
}
#[derive(Template)]
#[template(path = "deposit.html")]
pub struct DepositTemplate {
    pub error: Option<String>,
}

#[derive(Template)]
#[template(path = "place_order.html")]
pub struct PlaceOrderTemplate {
    pub error: Option<String>,
    pub account_balance: f64,
}

#[derive(Template)]
#[template(path = "orders.html")]
pub struct OrdersTemplate {
    pub orders: Vec<OrderDisplayData>,
}

#[derive(Template)]
#[template(path = "order_confirmation.html")]
pub struct OrderConfirmationTemplate {
    pub order_id: String,
    pub symbol: String,
    pub order_type: String,
    pub quantity: u64,
    pub price: f64,
    pub total_cost: f64,
}

impl OrderConfirmationTemplate {
    pub fn new(
        order_id: String,
        symbol: String,
        order_type: String,
        quantity: u64,
        price: f64,
    ) -> Self {
        let total_cost = price * (quantity as f64); // Cast u64 to f64 for multiplication
        Self {
            order_id,
            symbol,
            order_type,
            quantity,
            price,
            total_cost,
        }
    }
}

impl OrderDisplayData {
    pub fn from_order(order_id: OrderId, order: Order) -> Self {
        let order_type = match order.order_side {
            OrderSide::Buy => "Buy".to_string(),
            OrderSide::Sell => "Sell".to_string(),
        };

        let (order_kind, price) = match order.order_type {
            OrderType::Market => ("Market".to_string(), 0.0), // Market orders don't have a specific price
            OrderType::Limit(p) => ("Limit".to_string(), p),
        };

        let (status, status_tooltip) = match &order.status {
            OrderStatus::Queued => (
                "Queued".to_string(),
                Some("Order is waiting to be processed by the system".to_string()),
            ),
            OrderStatus::Pending => (
                "Pending".to_string(),
                Some("Order has been sent to the exchange but not yet executed".to_string()),
            ),
            OrderStatus::PartiallyFilled { amount_executed } => {
                let tooltip = format!(
                    "Only {} out of {} shares have been executed",
                    amount_executed, order.quantity
                );
                ("Partially Filled".to_string(), Some(tooltip))
            }
            OrderStatus::Filled { date } => {
                let tooltip = format!(
                    "Order was completely filled on {}",
                    date.format("%Y-%m-%d %H:%M")
                );
                ("Filled".to_string(), Some(tooltip))
            }
            OrderStatus::PendingCancel => (
                "Pending Cancel".to_string(),
                Some("Order cancellation is being processed".to_string()),
            ),
            OrderStatus::Cancelled => (
                "Cancelled".to_string(),
                Some("Order was cancelled by the user".to_string()),
            ),
            OrderStatus::Expired { date } => {
                let tooltip = format!("Order expired on {}", date.format("%Y-%m-%d %H:%M"));
                ("Expired".to_string(), Some(tooltip))
            }
            OrderStatus::Rejected { date } => {
                let tooltip = format!(
                    "Order was rejected by the system on {}",
                    date.format("%Y-%m-%d %H:%M")
                );
                ("Rejected".to_string(), Some(tooltip))
            }
        };

        let total = price * (order.quantity as f64);
        let date = order.date.format("%Y-%m-%d %H:%M").to_string();

        Self {
            id: order_id.to_string(),
            symbol: order.symbol,
            quantity: order.quantity,
            price,
            order_type,
            order_kind,
            status,
            date,
            total,
            status_tooltip,
        }
    }
}
