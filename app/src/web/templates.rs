use askama::Template;

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

// Temporary struct for order display until we implement full order system
#[derive(Clone)]
pub struct OrderDisplayData {
    pub id: String,
    pub symbol: String,
    pub quantity: u32,
    pub price: f64,
    pub status: String,
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
