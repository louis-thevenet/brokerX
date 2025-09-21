use color_eyre::Result;
use in_memory_adapter::InMemoryRepo;
use uuid::Uuid;
use std::collections::HashMap;

use crate::account::AccountId;

#[derive(Debug, Clone)]
pub struct User {
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub account_id: AccountId,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug)]
pub enum AuthError {
    UserNotFound,
    InvalidPassword,
    UserAlreadyExists,
    WeakPassword,
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::UserNotFound => write!(f, "User not found"),
            AuthError::InvalidPassword => write!(f, "Invalid password"),
            AuthError::UserAlreadyExists => write!(f, "User already exists"),
            AuthError::WeakPassword => write!(f, "Password is too weak"),
        }
    }
}

impl std::error::Error for AuthError {}

impl User {
    pub fn new(username: String, email: String, password: String, account_id: AccountId) -> Result<Self, AuthError> {
        if password.len() < 6 {
            return Err(AuthError::WeakPassword);
        }

        Ok(Self {
            username,
            email,
            password_hash: Self::hash_password(&password),
            account_id,
            created_at: chrono::Utc::now(),
        })
    }

    pub fn verify_password(&self, password: &str) -> bool {
        // In a real app, use bcrypt or similar
        // For now, we'll use a simple hash for demonstration
        self.password_hash == Self::hash_password(password)
    }

    fn hash_password(password: &str) -> String {
        // Simple hash for demonstration - use bcrypt in production!
        format!("hash_{}", password)
    }
}

pub type UserId = Uuid;

pub type UserRepo = InMemoryRepo<User, UserId>;

pub trait UserRepoExt {
    /// Creates a new user with the given details
    fn create_user(&mut self, username: String, email: String, password: String, account_id: AccountId) -> Result<UserId, AuthError>;
    
    /// Authenticates a user by username and password
    fn authenticate_user(&self, username: &str, password: &str) -> Result<AccountId, AuthError>;
    
    /// Gets a user by username
    fn get_user_by_username(&self, username: &str) -> Option<&User>;
    
    /// Checks if a username already exists
    fn username_exists(&self, username: &str) -> bool;
    
    /// Gets a user by account ID
    fn get_user_by_account_id(&self, account_id: &AccountId) -> Option<&User>;
}

impl UserRepoExt for UserRepo {
    fn create_user(&mut self, username: String, email: String, password: String, account_id: AccountId) -> Result<UserId, AuthError> {
        // Check if username already exists
        if self.username_exists(&username) {
            return Err(AuthError::UserAlreadyExists);
        }

        let user = User::new(username, email, password, account_id)?;
        let user_id = Uuid::new_v4();
        self.insert(user_id, user);
        
        Ok(user_id)
    }

    fn authenticate_user(&self, username: &str, password: &str) -> Result<AccountId, AuthError> {
        if let Some(user) = self.get_user_by_username(username) {
            if user.verify_password(password) {
                Ok(user.account_id)
            } else {
                Err(AuthError::InvalidPassword)
            }
        } else {
            Err(AuthError::UserNotFound)
        }
    }

    fn get_user_by_username(&self, username: &str) -> Option<&User> {
        self.values().find(|user| user.username == username)
    }

    fn username_exists(&self, username: &str) -> bool {
        self.get_user_by_username(username).is_some()
    }

    fn get_user_by_account_id(&self, account_id: &AccountId) -> Option<&User> {
        self.values().find(|user| user.account_id == *account_id)
    }
}
