use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: u32,
    pub username: String,
    pub email: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginForm {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct RegisterForm {
    pub username: String,
    pub email: String,
    pub password: String,
    pub confirm_password: String,
}

// Simple in-memory session storage (replace with proper session management in production)
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub type SessionStore = Arc<Mutex<HashMap<String, User>>>;

pub fn create_session_store() -> SessionStore {
    Arc::new(Mutex::new(HashMap::new()))
}

// Placeholder functions for authentication - replace with actual business logic
pub fn authenticate_user(username: &str, password: &str) -> Option<User> {
    // TODO: Replace with actual authentication logic
    if username == "demo" && password == "password" {
        Some(User {
            id: 1,
            username: username.to_string(),
            email: "demo@example.com".to_string(),
        })
    } else {
        None
    }
}

pub fn register_user(form: &RegisterForm) -> Result<User, String> {
    // TODO: Replace with actual registration logic
    if form.password != form.confirm_password {
        return Err("Passwords don't match".to_string());
    }

    if form.username.is_empty() || form.email.is_empty() || form.password.len() < 6 {
        return Err("Invalid form data".to_string());
    }

    Ok(User {
        id: 2, // TODO: Generate proper ID
        username: form.username.clone(),
        email: form.email.clone(),
    })
}
