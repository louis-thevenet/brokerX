use askama::Template;
use axum::{
    extract::{Form, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
};
use serde::Deserialize;

use crate::web::{AppState, templates::*};
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

// Handler functions
pub async fn home() -> Redirect {
    Redirect::permanent("/login")
}

pub async fn login_page() -> Result<Html<String>, StatusCode> {
    let template = LoginTemplate { error: None };
    match template.render() {
        Ok(html) => Ok(Html(html)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn login_submit(
    State(broker_x): State<AppState>,
    Form(form): Form<LoginForm>,
) -> Response {
    if form.email.is_empty() || form.password.is_empty() {
        let template = LoginTemplate {
            error: Some("Username and password are required".to_string()),
        };
        return match template.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        };
    }

    // Authenticate using the domain layer
    let broker = broker_x.lock().unwrap();
    if let Some(account_id) = broker
        .account_repo
        .authenticate(&form.email, &form.password)
    {
        if let Some(account) = broker.account_repo.get(&account_id) {
            println!(
                "Successful login for user: {} (Account ID: {})",
                form.email, account_id
            );
            println!(
                "Account: {} - Balance: ${:.2}",
                account.firstname,
                broker.account_repo.get_balance(&account_id).unwrap_or(0.0)
            );

            // TODO: Create session/JWT token here
            // For now, just redirect to a success page or dashboard
            Redirect::to("/login").into_response() // Change this to dashboard when implemented
        } else {
            let template = LoginTemplate {
                error: Some("Account not found".to_string()),
            };
            match template.render() {
                Ok(html) => Html(html).into_response(),
                Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            }
        }
    } else {
        let template = LoginTemplate {
            error: Some("Invalid username or password".to_string()),
        };
        match template.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
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
    State(broker_x): State<AppState>,
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
        let broker = broker_x.lock().unwrap();
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
    let account_id = {
        let mut broker = broker_x.lock().unwrap();
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

pub async fn logout() -> Redirect {
    // TODO: Clear session/cookies
    Redirect::to("/login")
}
