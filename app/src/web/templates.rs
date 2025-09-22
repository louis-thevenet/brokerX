use askama::Template;

#[derive(Template)]
#[template(path = "login.html")]
pub struct LoginTemplate {
    pub error: Option<String>,
    pub success: Option<String>,
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
