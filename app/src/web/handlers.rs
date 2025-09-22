use askama::Template;
use axum::{
    extract::{Form, Query, State},
    http::{header, StatusCode},
    response::{Html, IntoResponse, Redirect, Response},
};
use serde::Deserialize;
use uuid::Uuid;

use crate::web::{jwt, templates::*, AppState};
use domain::account::AccountRepoExt;

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
pub struct MfaQuery {
    pub challenge_id: String,
}

// Handler functions
pub async fn home() -> Redirect {
    Redirect::permanent("/dashboard")
}

pub async fn login_page() -> Result<Html<String>, StatusCode> {
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
    if form.email.is_empty() || form.password.is_empty() {
        let template = LoginTemplate {
            error: Some("Email and password are required".to_string()),
        };
        return match template.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        };
    }

    // First factor authentication using the domain layer
    let account_found = {
        let broker = app_state.lock().unwrap();
        broker
            .account_repo
            .authenticate(&form.email, &form.password)
            .is_some()
    };

    if !account_found {
        let template = LoginTemplate {
            error: Some("Invalid email or password".to_string()),
        };
        return match template.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        };
    }

    // Initiate MFA challenge
    let target_email = "louis.tvnt@gmail.com";
    let challenge_id_result = {
        let broker = app_state.lock().unwrap();
        // TODO: tokio::task::spawn_blocking ?
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(broker.mfa_service.initiate_mfa(&form.email))
        })
    };

    match challenge_id_result {
        Ok(challenge_id) => {
            // Redirect to MFA verification page
            Redirect::to(&format!("/verify-mfa?challenge_id={challenge_id}")).into_response()
        }
        Err(e) => {
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
    // Basic validation
    if form.password != form.confirm_password {
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
    {
        let broker = app_state.lock().unwrap();
        if broker.account_repo.email_exists(&form.email) {
            let template = RegisterTemplate {
                error: Some("Username already exists".to_string()),
            };
            return match template.render() {
                Ok(html) => Html(html).into_response(),
                Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            };
        }
    }

    // Create account in the domain layer
    let _account_id = {
        let mut broker = app_state.lock().unwrap();
        let account_id = broker.account_repo.create_account(
            form.firstname.clone(),
            form.surname.clone(),
            form.email.clone(),
            form.password.clone(),
            1000.0, // TODO: change
        );

        println!(
            "Created new account for user: {} (ID: {})",
            form.email, account_id
        );
        println!(
            "Account created with email: {} and initial balance: $1000.00",
            form.email
        );

        account_id
    };

    println!("Registration successful for user: {}", form.email);

    // Redirect to login page with success
    Redirect::to("/login").into_response()
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

/// Dashboard handler - requires authentication
pub async fn dashboard(
    State(app_state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    // Extract user claims from request (added by auth middleware)
    let claims = match request.extensions().get::<jwt::Claims>() {
        Some(claims) => claims,
        None => return Redirect::to("/login").into_response(),
    };

    // Get user account from domain layer
    let account_id = match Uuid::parse_str(&claims.sub) {
        Ok(id) => id,
        Err(_) => return Redirect::to("/login").into_response(),
    };

    let broker = app_state.lock().unwrap();
    let account = match broker.account_repo.get(&account_id) {
        Some(account) => account,
        None => return Redirect::to("/login").into_response(),
    };

    let balance = broker.account_repo.get_balance(&account_id).unwrap_or(0.0);

    // Create dashboard template (we'll need to create this)
    let template = DashboardTemplate {
        username: &claims.email,
        firstname: &account.firstname,
        surname: &account.surname,
        email: &account.email,
        account_balance: balance,
        recent_orders: vec![], // Empty for now, will be populated when order system is implemented
    };

    match template.render() {
        Ok(html) => Html(html).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
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
                    // Get the account using the email from the challenge
                    let (account_id, email) = {
                        let broker = app_state.lock().unwrap();
                        match broker
                            .account_repo
                            .get_account_id_by_email(&challenge.user_email)
                        {
                            Some(account_id) => {
                                if let Some(account) = broker.account_repo.get(&account_id) {
                                    (account_id, account.email.clone())
                                } else {
                                    let template = MfaVerifyTemplate {
                                        challenge_id: form.challenge_id,
                                        error: Some("Account not found".to_string()),
                                    };
                                    return match template.render() {
                                        Ok(html) => Html(html).into_response(),
                                        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
                                    };
                                }
                            }
                            None => {
                                let template = MfaVerifyTemplate {
                                    challenge_id: form.challenge_id,
                                    error: Some("User account not found".to_string()),
                                };
                                return match template.render() {
                                    Ok(html) => Html(html).into_response(),
                                    Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
                                };
                            }
                        }
                    };

                    // Create JWT token
                    if let Ok(token) = jwt::create_jwt(account_id, email) {
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
                        error: Some(format!("Challenge error: {}", e)),
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
                error: Some(format!("Verification failed: {}", e)),
            };
            match template.render() {
                Ok(html) => Html(html).into_response(),
                Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            }
        }
    }
}
