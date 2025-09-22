use async_trait::async_trait;
use color_eyre::Result;
use in_memory_adapter::InMemoryRepo;
use uuid::Uuid;

use crate::mfa::{MfaError, MfaProvider, MfaService};

#[derive(Debug, Clone)]
pub struct User {
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub firstname: String,
    pub surname: String,
    pub balance: f64,
    pub is_verified: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug)]
pub struct NotEnoughMoneyError;

#[derive(Debug)]
pub enum AuthError {
    UserNotFound,
    InvalidPassword,
    UserAlreadyExists,
    WeakPassword,
    MfaRequired,
    MfaFailed(MfaError),
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::UserNotFound => write!(f, "User not found"),
            AuthError::InvalidPassword => write!(f, "Invalid password"),
            AuthError::UserAlreadyExists => write!(f, "User already exists"),
            AuthError::WeakPassword => write!(f, "Password is too weak"),
            AuthError::MfaRequired => write!(f, "Multi-factor authentication required"),
            AuthError::MfaFailed(err) => write!(f, "MFA failed: {}", err),
        }
    }
}

impl std::error::Error for AuthError {}

impl User {
    pub fn new(
        username: String,
        email: String,
        password: String,
        firstname: String,
        surname: String,
        initial_balance: f64,
    ) -> Result<Self, AuthError> {
        if password.len() < 6 {
            return Err(AuthError::WeakPassword);
        }

        Ok(Self {
            username,
            email,
            password_hash: Self::hash_password(&password),
            firstname,
            surname,
            balance: initial_balance,
            is_verified: false,
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

    /// Deposit money into the user's account
    pub fn deposit(&mut self, amount: f64) {
        self.balance += amount;
    }

    /// Withdraw money from the user's account
    pub fn withdraw(&mut self, amount: f64) -> Result<(), NotEnoughMoneyError> {
        if self.balance < amount {
            return Err(NotEnoughMoneyError);
        }
        self.balance -= amount;
        Ok(())
    }

    /// Get the current balance
    pub fn get_balance(&self) -> f64 {
        self.balance
    }

    /// Mark the user as verified
    pub fn verify_email(&mut self) {
        self.is_verified = true;
    }

    /// Check if the user's email is verified
    pub fn is_email_verified(&self) -> bool {
        self.is_verified
    }
}

pub type UserId = Uuid;

pub type UserRepo = InMemoryRepo<User, UserId>;

#[async_trait]
pub trait UserRepoExt {
    /// Creates a new user with the given details
    fn create_user(
        &mut self,
        username: String,
        email: String,
        password: String,
        firstname: String,
        surname: String,
        initial_balance: f64,
    ) -> Result<UserId, AuthError>;

    /// Authenticates a user by username and password (first factor)
    fn authenticate_user(&self, username: &str, password: &str) -> Result<UserId, AuthError>;

    /// Initiates MFA for a user after successful first-factor authentication
    async fn initiate_mfa<P: MfaProvider>(
        &self,
        username: &str,
        mfa_service: &MfaService<P>,
    ) -> Result<String, AuthError>;

    /// Completes authentication by verifying MFA
    async fn complete_mfa_authentication<P: MfaProvider>(
        &self,
        challenge_id: &str,
        code: &str,
        mfa_service: &MfaService<P>,
    ) -> Result<bool, AuthError>;

    /// Gets a user by username
    fn get_user_by_username(&self, username: &str) -> Option<&User>;

    /// Gets a user by email
    fn get_user_by_email(&self, email: &str) -> Option<&User>;

    /// Gets a user by user ID
    fn get_user_by_id(&self, user_id: &UserId) -> Option<&User>;

    /// Checks if a username already exists
    fn username_exists(&self, username: &str) -> bool;

    /// Checks if an email already exists
    fn email_exists(&self, email: &str) -> bool;

    /// Deposits money to a user's account
    fn deposit_to_user(&mut self, user_id: &UserId, amount: f64) -> Result<(), &'static str>;

    /// Withdraws money from a user's account
    fn withdraw_from_user(&mut self, user_id: &UserId, amount: f64) -> Result<(), &'static str>;

    /// Gets the balance of a user's account
    fn get_user_balance(&self, user_id: &UserId) -> Option<f64>;

    /// Marks a user as verified by email
    fn verify_user_email(&mut self, user_id: &UserId) -> Result<(), &'static str>;

    /// Checks if a user is verified
    fn is_user_verified(&self, user_id: &UserId) -> Option<bool>;
}

#[async_trait]
impl UserRepoExt for UserRepo {
    fn create_user(
        &mut self,
        username: String,
        email: String,
        password: String,
        firstname: String,
        surname: String,
        initial_balance: f64,
    ) -> Result<UserId, AuthError> {
        // Check if username already exists
        if self.username_exists(&username) {
            return Err(AuthError::UserAlreadyExists);
        }

        // Check if email already exists
        if self.email_exists(&email) {
            return Err(AuthError::UserAlreadyExists);
        }

        let user = User::new(
            username,
            email,
            password,
            firstname,
            surname,
            initial_balance,
        )?;
        let user_id = Uuid::new_v4();
        self.insert(user_id, user);

        Ok(user_id)
    }

    fn authenticate_user(&self, username: &str, password: &str) -> Result<UserId, AuthError> {
        if let Some((user_id, user)) = self
            .iter()
            .find(|(_, user)| user.username == username || user.email == username)
        {
            if !user.is_verified {
                return Err(AuthError::UserNotFound); // Treat unverified users as not found for security
            }
            if user.verify_password(password) {
                Ok(*user_id)
            } else {
                Err(AuthError::InvalidPassword)
            }
        } else {
            Err(AuthError::UserNotFound)
        }
    }

    async fn initiate_mfa<P: MfaProvider>(
        &self,
        username: &str,
        mfa_service: &MfaService<P>,
    ) -> Result<String, AuthError> {
        let user = self
            .get_user_by_username(username)
            .or_else(|| self.get_user_by_email(username))
            .ok_or(AuthError::UserNotFound)?;

        mfa_service
            .initiate_mfa(&user.email)
            .await
            .map_err(AuthError::MfaFailed)
    }

    async fn complete_mfa_authentication<P: MfaProvider>(
        &self,
        challenge_id: &str,
        code: &str,
        mfa_service: &MfaService<P>,
    ) -> Result<bool, AuthError> {
        mfa_service
            .verify_mfa(challenge_id, code)
            .await
            .map_err(AuthError::MfaFailed)
    }

    fn get_user_by_username(&self, username: &str) -> Option<&User> {
        self.iter().find_map(|(_, user)| {
            if user.username == username {
                Some(user)
            } else {
                None
            }
        })
    }

    fn get_user_by_email(&self, email: &str) -> Option<&User> {
        self.iter().find_map(|(_, user)| {
            if user.email == email {
                Some(user)
            } else {
                None
            }
        })
    }

    fn get_user_by_id(&self, user_id: &UserId) -> Option<&User> {
        self.get(user_id)
    }

    fn username_exists(&self, username: &str) -> bool {
        self.get_user_by_username(username).is_some()
    }

    fn email_exists(&self, email: &str) -> bool {
        self.get_user_by_email(email).is_some()
    }

    fn deposit_to_user(&mut self, user_id: &UserId, amount: f64) -> Result<(), &'static str> {
        if let Some(user) = self.get_mut(user_id) {
            user.deposit(amount);
            Ok(())
        } else {
            Err("User not found")
        }
    }

    fn withdraw_from_user(&mut self, user_id: &UserId, amount: f64) -> Result<(), &'static str> {
        if let Some(user) = self.get_mut(user_id) {
            user.withdraw(amount).map_err(|_| "Insufficient funds")
        } else {
            Err("User not found")
        }
    }

    fn get_user_balance(&self, user_id: &UserId) -> Option<f64> {
        self.get(user_id).map(|user| user.balance)
    }

    fn verify_user_email(&mut self, user_id: &UserId) -> Result<(), &'static str> {
        if let Some(user) = self.get_mut(user_id) {
            user.verify_email();
            Ok(())
        } else {
            Err("User not found")
        }
    }

    fn is_user_verified(&self, user_id: &UserId) -> Option<bool> {
        self.get(user_id).map(|user| user.is_email_verified())
    }
}
