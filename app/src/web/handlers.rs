use askama::Template;
use axum::{
    extract::{Form, FromRequest, Query, State},
    http::{header, StatusCode},
    response::{Html, IntoResponse, Redirect, Response},
};
use serde::Deserialize;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use super::templates::{DepositTemplate, PlaceOrderTemplate};
use crate::web::{
    jwt,
    templates::{
        DashboardTemplate, LoginTemplate, MfaVerifyTemplate, RegisterTemplate,
        RegistrationVerifyTemplate,
    },
    AppState,
};
use domain::order::{Order, OrderRepoExt};
use domain::user::{AuthError, User, UserRepoExt};

#[derive(Deserialize)]
pub struct LoginForm {
    pub email: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct RegisterForm {
    pub firstname: String,
    pub surname: String,
    pub email: String,
    pub password: String,
    pub confirm_password: String,
}

#[derive(Deserialize)]
pub struct MfaVerifyForm {
    pub challenge_id: String,
    pub code: String,
}

#[derive(Deserialize)]
pub struct RegistrationVerifyForm {
    pub challenge_id: String,
    pub user_id: String,
    pub code: String,
}

#[derive(Deserialize)]
pub struct MfaQuery {
    pub challenge_id: String,
}

#[derive(Deserialize)]
pub struct RegistrationVerifyQuery {
    pub challenge_id: String,
    pub user_id: String,
}

#[derive(Deserialize)]
pub struct LoginQuery {}

#[derive(Deserialize)]
pub struct ResendMfaQuery {
    pub challenge_id: String,
}

#[derive(Deserialize)]
pub struct DepositForm {
    pub amount: String,
}

#[derive(Deserialize)]
pub struct PlaceOrderForm {
    pub symbol: String,
    pub side: String,       // "buy" or "sell"
    pub order_type: String, // "market" or "limit"
    pub quantity: String,
    pub price: String,
}

// Handler functions
pub async fn home() -> Redirect {
    Redirect::permanent("/dashboard")
}

pub async fn login_page(Query(_params): Query<LoginQuery>) -> Result<Html<String>, StatusCode> {
    let template = LoginTemplate { error: None };
    match template.render() {
        Ok(html) => Ok(Html(html)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn login_submit(
    State(app_state): State<AppState>,
    Form(form): Form<LoginForm>,
) -> Response {
    info!("Login attempt for email: {}", form.email);

    if form.email.is_empty() || form.password.is_empty() {
        warn!(
            "Login attempt with empty credentials for email: {}",
            form.email
        );
        let template = LoginTemplate {
            error: Some("Email and password are required".to_string()),
        };
        return match template.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        };
    }

    // First factor authentication using the domain layer
    let user_id_found = {
        let broker = app_state.lock().unwrap();
        match broker
            .user_repo
            .authenticate_user(&form.email, &form.password)
        {
            Ok(_) => true,
            Err(AuthError::NotVerified(user_id)) => {
                drop(broker);
                // start email verification MFA process
                return registration_mfa(app_state.clone(), &form.email, user_id);
            }
            Err(e) => {
                warn!("Authentication failed for email: {} - {}", form.email, e);
                false
            }
        }
    };

    if !user_id_found {
        warn!("Failed authentication attempt for email: {}", form.email);
        let template = LoginTemplate {
            error: Some("Invalid email or password".to_string()),
        };
        return match template.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        };
    }

    debug!(
        "First factor authentication successful for email: {}",
        form.email
    );

    let challenge_id_result = {
        let broker = app_state.lock().unwrap();
        // TODO: tokio::task::spawn_blocking ?
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(broker.mfa_service.initiate_mfa(&form.email))
        })
    };

    match challenge_id_result {
        Ok(challenge_id) => {
            info!(
                "MFA challenge initiated for email: {}, challenge_id: {}",
                form.email, challenge_id
            );
            // Redirect to MFA verification page
            Redirect::to(&format!("/verify-mfa?challenge_id={challenge_id}")).into_response()
        }
        Err(e) => {
            error!(
                "Failed to initiate MFA for email: {}, error: {}",
                form.email, e
            );
            let template = LoginTemplate {
                error: Some(format!("Failed to send verification code: {e}")),
            };
            match template.render() {
                Ok(html) => Html(html).into_response(),
                Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            }
        }
    }
}

pub async fn register_page() -> Result<Html<String>, StatusCode> {
    let template = RegisterTemplate { error: None };
    match template.render() {
        Ok(html) => Ok(Html(html)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn register_submit(
    State(app_state): State<AppState>,
    Form(form): Form<RegisterForm>,
) -> Response {
    info!("Registration attempt for email: {}", form.email);

    // Basic validation
    if form.password != form.confirm_password {
        warn!(
            "Registration failed for email: {} - passwords do not match",
            form.email
        );
        let template = RegisterTemplate {
            error: Some("Passwords do not match".to_string()),
        };
        return match template.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        };
    }

    if form.firstname.is_empty()
        || form.surname.is_empty()
        || form.email.is_empty()
        || form.password.is_empty()
    {
        warn!(
            "Registration failed for email: {} - missing required fields",
            form.email
        );
        let template = RegisterTemplate {
            error: Some("All fields are required".to_string()),
        };
        return match template.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        };
    }

    if form.password.len() < 6 {
        let template = RegisterTemplate {
            error: Some("Password must be at least 6 characters long".to_string()),
        };
        return match template.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        };
    }

    // Check if username already exists using the domain layer
    // If so, is it verified yet?

    let user_id = {
        let mut broker = app_state.lock().unwrap();

        match broker.user_repo.get_user_by_email(&form.email) {
            Some(u) if !u.is_verified => {
                // just skip domain user creation and proceed to MFA
                warn!(
                    "Registration attempt for existing unverified email: {}",
                    form.email
                );
                *broker.user_repo.get_user_id(&form.email).unwrap() // we know it exists
            }
            Some(_u) => {
                let template = RegisterTemplate {
                    error: Some("Email already exists".to_string()),
                };
                return match template.render() {
                    Ok(html) => Html(html).into_response(),
                    Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
                };
            }
            None => {
                // Create user in the domain layer

                match broker.user_repo.create_user(
                    form.email.clone(),
                    form.password.clone(),
                    form.firstname.clone(),
                    form.surname.clone(),
                    1000.0, // TODO: change
                ) {
                    Ok(user_id) => {
                        debug!("Created new user: {} (ID: {})", form.email, user_id);
                        user_id
                    }
                    Err(e) => {
                        let template = RegisterTemplate {
                            error: Some(format!("Registration failed: {e}")),
                        };
                        return match template.render() {
                            Ok(html) => Html(html).into_response(),
                            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
                        };
                    }
                }
            }
        }
    };
    registration_mfa(app_state, &form.email, user_id)
}

fn registration_mfa(
    app_state: std::sync::Arc<std::sync::Mutex<domain::core::BrokerX>>,
    email: &str,
    user_id: Uuid,
) -> axum::http::Response<axum::body::Body> {
    // Initiate MFA for email verification
    let challenge_id_result = {
        let broker = app_state.lock().unwrap();
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(broker.mfa_service.initiate_mfa(email))
        })
    };

    match challenge_id_result {
        Ok(challenge_id) => {
            info!(
                "Registration MFA challenge initiated for email: {}, challenge_id: {}",
                email, challenge_id
            );
            // Redirect to registration MFA verification page
            Redirect::to(&format!(
                "/verify-registration?challenge_id={challenge_id}&user_id={user_id}"
            ))
            .into_response()
        }
        Err(e) => {
            error!(
                "Failed to initiate registration MFA for email: {}, error: {}",
                email, e
            );
            // Delete the created user since verification failed
            {
                let mut broker = app_state.lock().unwrap();
                let _ = broker.user_repo.remove(&user_id);
            }
            let template = RegisterTemplate {
                error: Some(format!("Failed to send verification email: {e}")),
            };
            match template.render() {
                Ok(html) => Html(html).into_response(),
                Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            }
        }
    }
}

pub async fn logout() -> Response {
    // Clear JWT cookie and redirect to login
    let mut response = Redirect::to("/login").into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        jwt::create_logout_cookie().parse().unwrap(),
    );
    response
}
fn check_token_and_execute(
    app_state: State<AppState>,
    request: axum::extract::Request,
    handler: fn(State<AppState>, User, axum::extract::Request) -> Response,
) -> Response {
    // Extract user claims from request
    let Some(claims) = request.extensions().get::<jwt::Claims>() else {
        return Redirect::to("/login").into_response();
    };

    // Get user from domain layer
    let Ok(user_id) = Uuid::parse_str(&claims.subject) else {
        return Redirect::to("/login").into_response();
    };

    let broker = app_state.lock().unwrap();
    let Some(user) = broker.user_repo.get(&user_id).cloned() else {
        return Redirect::to("/login").into_response();
    };

    drop(broker);
    handler(app_state, user, request)
}
/// Dashboard handler - requires authentication
pub async fn dashboard(app_state: State<AppState>, request: axum::extract::Request) -> Response {
    check_token_and_execute(app_state, request, |_app_state, user, _request| {
        // Create dashboard template (we'll need to create this)
        let template = DashboardTemplate {
            username: &user.email,
            firstname: &user.firstname,
            surname: &user.surname,
            email: &user.email,
            account_balance: user.balance,
            recent_orders: vec![], // TODO: Empty for now, will be populated when order system is implemented
        };

        match template.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    })
}

pub async fn mfa_verify_page(Query(params): Query<MfaQuery>) -> Result<Html<String>, StatusCode> {
    let template = MfaVerifyTemplate {
        challenge_id: params.challenge_id,
        error: None,
    };
    match template.render() {
        Ok(html) => Ok(Html(html)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn mfa_verify_submit(
    State(app_state): State<AppState>,
    Form(form): Form<MfaVerifyForm>,
) -> Response {
    if form.code.is_empty() || form.code.len() != 6 {
        let template = MfaVerifyTemplate {
            challenge_id: form.challenge_id,
            error: Some("Please enter a valid 6-digit code".to_string()),
        };
        return match template.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        };
    }

    // Verify the MFA code
    let verification_result = {
        let broker = app_state.lock().unwrap();
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(
                broker
                    .mfa_service
                    .verify_mfa(&form.challenge_id, &form.code),
            )
        })
    };

    match verification_result {
        Ok(true) => {
            // MFA verified successfully, now get the challenge to retrieve user info
            let challenge = {
                let broker = app_state.lock().unwrap();
                tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current()
                        .block_on(broker.mfa_service.get_challenge(&form.challenge_id))
                })
            };

            match challenge {
                Ok(challenge) => {
                    // Get the user using the email from the challenge
                    let (user_id, email) = {
                        let broker = app_state.lock().unwrap();
                        if let Some(user) =
                            broker.user_repo.get_user_by_email(&challenge.user_email)
                        {
                            // Find the user ID by iterating through the repo
                            if let Some((id, _)) = broker
                                .user_repo
                                .iter()
                                .find(|(_, stored_user)| stored_user.email == user.email)
                            {
                                (*id, user.email.clone())
                            } else {
                                let template = MfaVerifyTemplate {
                                    challenge_id: form.challenge_id,
                                    error: Some("User ID not found".to_string()),
                                };
                                return match template.render() {
                                    Ok(html) => Html(html).into_response(),
                                    Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
                                };
                            }
                        } else {
                            let template = MfaVerifyTemplate {
                                challenge_id: form.challenge_id,
                                error: Some("User account not found".to_string()),
                            };
                            return match template.render() {
                                Ok(html) => Html(html).into_response(),
                                Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
                            };
                        }
                    };

                    // Create JWT token
                    if let Ok(token) = jwt::create_jwt(user_id, email) {
                        // Create response with auth cookie
                        let mut response = Redirect::to("/dashboard").into_response();
                        response.headers_mut().insert(
                            header::SET_COOKIE,
                            jwt::create_auth_cookie(&token).parse().unwrap(),
                        );
                        response
                    } else {
                        let template = MfaVerifyTemplate {
                            challenge_id: form.challenge_id,
                            error: Some("Failed to create session".to_string()),
                        };
                        match template.render() {
                            Ok(html) => Html(html).into_response(),
                            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
                        }
                    }
                }
                Err(e) => {
                    let template = MfaVerifyTemplate {
                        challenge_id: form.challenge_id,
                        error: Some(format!("Challenge error: {e}")),
                    };
                    match template.render() {
                        Ok(html) => Html(html).into_response(),
                        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
                    }
                }
            }
        }
        Ok(false) => {
            let template = MfaVerifyTemplate {
                challenge_id: form.challenge_id,
                error: Some("Invalid verification code".to_string()),
            };
            match template.render() {
                Ok(html) => Html(html).into_response(),
                Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            }
        }
        Err(e) => {
            let template = MfaVerifyTemplate {
                challenge_id: form.challenge_id,
                error: Some(format!("Verification failed: {e}")),
            };
            match template.render() {
                Ok(html) => Html(html).into_response(),
                Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            }
        }
    }
}

pub async fn registration_verify_page(
    Query(params): Query<RegistrationVerifyQuery>,
) -> Result<Html<String>, StatusCode> {
    let template = RegistrationVerifyTemplate {
        challenge_id: params.challenge_id,
        user_id: params.user_id,
        error: None,
    };
    match template.render() {
        Ok(html) => Ok(Html(html)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn registration_verify_submit(
    State(app_state): State<AppState>,
    Form(form): Form<RegistrationVerifyForm>,
) -> Response {
    if form.code.is_empty() || form.code.len() != 6 {
        let template = RegistrationVerifyTemplate {
            challenge_id: form.challenge_id,
            user_id: form.user_id,
            error: Some("Please enter a valid 6-digit code".to_string()),
        };
        return match template.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        };
    }

    // Parse user ID
    let user_id = if let Ok(id) = Uuid::parse_str(&form.user_id) {
        id
    } else {
        let template = RegistrationVerifyTemplate {
            challenge_id: form.challenge_id,
            user_id: form.user_id,
            error: Some("Invalid user ID".to_string()),
        };
        return match template.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        };
    };

    // Verify the MFA code
    let verification_result = {
        let broker = app_state.lock().unwrap();
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(
                broker
                    .mfa_service
                    .verify_mfa(&form.challenge_id, &form.code),
            )
        })
    };

    match verification_result {
        Ok(true) => {
            // MFA verified successfully, mark user as verified
            let verification_success = {
                let mut broker = app_state.lock().unwrap();
                broker.user_repo.verify_user_email(&user_id).is_ok()
            };

            if verification_success {
                info!("Email verification successful for user ID: {}", user_id);
                // Redirect to login page with success message
                // For now, we'll redirect to login page with a query parameter
                Redirect::to("/login?registered=true").into_response()
            } else {
                let template = RegistrationVerifyTemplate {
                    challenge_id: form.challenge_id,
                    user_id: form.user_id,
                    error: Some("Failed to verify user account".to_string()),
                };
                match template.render() {
                    Ok(html) => Html(html).into_response(),
                    Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
                }
            }
        }
        Ok(false) => {
            let template = RegistrationVerifyTemplate {
                challenge_id: form.challenge_id,
                user_id: form.user_id,
                error: Some("Invalid verification code".to_string()),
            };
            match template.render() {
                Ok(html) => Html(html).into_response(),
                Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            }
        }
        Err(e) => {
            let template = RegistrationVerifyTemplate {
                challenge_id: form.challenge_id,
                user_id: form.user_id,
                error: Some(format!("Verification failed: {e}")),
            };
            match template.render() {
                Ok(html) => Html(html).into_response(),
                Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            }
        }
    }
}

pub async fn resend_mfa(
    Query(params): Query<ResendMfaQuery>,
    State(app_state): State<AppState>,
) -> Response {
    // Get the original challenge to extract the user email
    let challenge_result = {
        let broker = app_state.lock().unwrap();
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(broker.mfa_service.get_challenge(&params.challenge_id))
        })
    };

    match challenge_result {
        Ok(challenge) => {
            // Initiate a new MFA challenge for the same user
            let new_challenge_id_result = {
                let broker = app_state.lock().unwrap();
                tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current()
                        .block_on(broker.mfa_service.initiate_mfa(&challenge.user_email))
                })
            };

            match new_challenge_id_result {
                Ok(new_challenge_id) => {
                    info!(
                        "MFA code resent for email: {}, new challenge_id: {}",
                        challenge.user_email, new_challenge_id
                    );
                    // Redirect to MFA verification page with new challenge ID
                    Redirect::to(&format!("/verify-mfa?challenge_id={new_challenge_id}"))
                        .into_response()
                }
                Err(e) => {
                    error!(
                        "Failed to resend MFA for email: {}, error: {}",
                        challenge.user_email, e
                    );
                    // Redirect back to the original MFA page with error
                    let template = MfaVerifyTemplate {
                        challenge_id: params.challenge_id,
                        error: Some(format!("Failed to resend verification code: {e}")),
                    };
                    match template.render() {
                        Ok(html) => Html(html).into_response(),
                        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
                    }
                }
            }
        }
        Err(e) => {
            error!(
                "Failed to get challenge for resend: challenge_id={}, error={}",
                params.challenge_id, e
            );
            // Redirect back to login if challenge is invalid/expired
            Redirect::to("/login").into_response()
        }
    }
}
pub async fn deposit_page(app_state: State<AppState>, request: axum::extract::Request) -> Response {
    check_token_and_execute(app_state, request, |_app_state, _user, _request| {
        let template = DepositTemplate { error: None };
        match template.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    })
}
pub async fn deposit_submit(
    State(app_state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let (parts, body) = request.into_parts(); // get form

    // Check for authentication token
    let Some(claims) = parts.extensions.get::<jwt::Claims>() else {
        return Redirect::to("/login").into_response();
    };

    // Get user from domain layer
    let Ok(user_id) = Uuid::parse_str(&claims.subject) else {
        return Redirect::to("/login").into_response();
    };

    let user = {
        let broker = app_state.lock().unwrap();
        let Some(user) = broker.user_repo.get(&user_id).cloned() else {
            return Redirect::to("/login").into_response();
        };
        user
    };

    let request = axum::extract::Request::from_parts(parts, body);
    let Ok(Form(form)) = Form::<DepositForm>::from_request(request, &app_state).await else {
        let template = DepositTemplate {
            error: Some("Invalid form data".to_string()),
        };
        return match template.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        };
    };

    info!(
        "Deposit attempt for user: {} amount: {}",
        user.email, form.amount
    );

    // Parse and validate amount
    let amount: f64 = match form.amount.parse() {
        Ok(amt) if amt > 0.0 => amt,
        _ => {
            let template = DepositTemplate {
                error: Some("Please enter a valid positive amount".to_string()),
            };
            return match template.render() {
                Ok(html) => Html(html).into_response(),
                Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            };
        }
    };

    // Process the deposit
    let deposit_result = {
        let mut broker = app_state.lock().unwrap();
        // TODO: Implement proper deposit logic in domain layer
        if let Some(user_mut) = broker.user_repo.get_mut(&user_id) {
            user_mut.balance += amount;
            Ok(())
        } else {
            Err("User not found")
        }
    };

    match deposit_result {
        Ok(()) => {
            info!(
                "Deposit successful for user: {} amount: {}",
                user.email, amount
            );
            Redirect::to("/dashboard").into_response()
        }
        Err(e) => {
            error!("Deposit failed for user: {} error: {}", user.email, e);
            let template = DepositTemplate {
                error: Some(format!("Deposit failed: {e}")),
            };
            match template.render() {
                Ok(html) => Html(html).into_response(),
                Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            }
        }
    }
}
pub async fn place_order_page(
    app_state: State<AppState>,
    request: axum::extract::Request,
) -> Response {
    check_token_and_execute(app_state, request, |_app_state, user, _request| {
        let template = PlaceOrderTemplate {
            error: None,
            account_balance: user.balance,
        };
        match template.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    })
}
#[allow(clippy::too_many_lines)]
pub async fn place_order_submit(
    State(app_state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let (parts, body) = request.into_parts(); // get form

    // Check for authentication token
    let Some(claims) = parts.extensions.get::<jwt::Claims>() else {
        return Redirect::to("/login").into_response();
    };

    // Get user from domain layer
    let Ok(user_id) = Uuid::parse_str(&claims.subject) else {
        return Redirect::to("/login").into_response();
    };

    let user = {
        let broker = app_state.lock().unwrap();
        let Some(user) = broker.user_repo.get(&user_id).cloned() else {
            return Redirect::to("/login").into_response();
        };
        user
    };

    let request = axum::extract::Request::from_parts(parts, body);
    let Ok(Form(form)) = Form::<PlaceOrderForm>::from_request(request, &app_state).await else {
        let template = PlaceOrderTemplate {
            error: Some("Invalid form data".to_string()),
            account_balance: user.balance,
        };
        return match template.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        };
    };
    info!(
        "Place order attempt for user: {} symbol: {} type: {} quantity: {} price: {}",
        user.email, form.symbol, form.order_type, form.quantity, form.price
    );
    let quantity = match form.quantity.parse::<u64>() {
        Ok(q) if q > 0 => q,
        _ => {
            let template = PlaceOrderTemplate {
                error: Some("Please enter a valid positive quantity".to_string()),
                account_balance: user.balance,
            };
            return match template.render() {
                Ok(html) => Html(html).into_response(),
                Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            };
        }
    };

    let order_side = match form.side.as_str() {
        "buy" => domain::order::OrderSide::Buy,
        "sell" => domain::order::OrderSide::Sell,
        _ => {
            let template = PlaceOrderTemplate {
                error: Some("Invalid order side".to_string()),
                account_balance: user.balance,
            };
            return match template.render() {
                Ok(html) => Html(html).into_response(),
                Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            };
        }
    };
    let order_type = match form.order_type.as_str() {
        "market" => domain::order::OrderType::Market,
        "limit" => {
            let limit = match form.price.parse::<f64>() {
                Ok(p) if p > 0.0 => p,
                _ => {
                    let template = PlaceOrderTemplate {
                        error: Some("Please enter a valid positive price".to_string()),
                        account_balance: user.balance,
                    };
                    return match template.render() {
                        Ok(html) => Html(html).into_response(),
                        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
                    };
                }
            };
            domain::order::OrderType::Limit(limit)
        }
        _ => {
            let template = PlaceOrderTemplate {
                error: Some("Invalid order type".to_string()),
                account_balance: user.balance,
            };
            return match template.render() {
                Ok(html) => Html(html).into_response(),
                Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            };
        }
    };
    {
        let mut broker = app_state.lock().unwrap();
        match broker.create_order(
            user_id,
            form.symbol.clone(),
            quantity,
            order_side,
            order_type,
        ) {
            Ok(_) => {
                info!(
                    "Order successfully sent for user: {} symbol: {} type: {} quantity: {} price: {}",
                    user.email, form.symbol, form.order_type, form.quantity, form.price
                );
                Redirect::to("/dashboard").into_response()
            }

            Err(e) => {
                error!(
                    "Order placement failed for user: {} error: {}",
                    user.email, e
                );
                let template = PlaceOrderTemplate {
                    error: Some(format!("Order placement failed: {e}")),
                    account_balance: user.balance,
                };
                match template.render() {
                    Ok(html) => Html(html).into_response(),
                    Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
                }
            }
        }
    }
}
