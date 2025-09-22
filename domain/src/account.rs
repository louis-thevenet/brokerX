use color_eyre::Result;
use in_memory_adapter::InMemoryRepo;
use uuid::Uuid;

#[derive(Debug)]
pub struct Account {
    pub firstname: String,
    pub surname: String,
    pub email: String,
    password_hash: String,
    balance: f64,
}
pub struct NotEnoughMoneyError;
impl Account {
    #[must_use]
    pub fn new(
        email: String,
        firstname: String,
        surname: String,
        password: String,
        balance: f64,
    ) -> Self {
        Self {
            email,
            surname,
            firstname,
            password_hash: Self::hash_password(&password),
            balance,
        }
    }

    /// Hash a password (simple implementation - in production use bcrypt or similar)
    fn hash_password(password: &str) -> String {
        // For now, just prepend "hash:" to indicate it's "hashed"
        // In production, use bcrypt, scrypt, or argon2
        format!("hash:{}", password)
    }

    /// Verify a password against the stored hash
    pub fn verify_password(&self, password: &str) -> bool {
        let expected_hash = Self::hash_password(password);
        self.password_hash == expected_hash
    }

    pub fn deposit(&mut self, amount: f64) {
        self.balance += amount;
    }

    pub fn withdraw(&mut self, amount: f64) -> Result<(), NotEnoughMoneyError> {
        if self.balance < amount {
            return Err(NotEnoughMoneyError);
        }
        self.balance -= amount;
        Ok(())
    }
}

pub type AccountId = Uuid;

pub type AccountRepo = InMemoryRepo<Account, AccountId>;

pub trait AccountRepoExt {
    /// Creates a new account with authentication credentials and initial balance.
    fn create_account(
        &mut self,
        firstname: String,
        surname: String,
        email: String,
        password: String,
        initial_balance: f64,
    ) -> AccountId;

    /// Authenticate a user and return their account ID if successful
    fn authenticate(&self, username: &str, password: &str) -> Option<AccountId>;

    /// Get account ID by email address
    fn get_account_id_by_email(&self, email: &str) -> Option<AccountId>;

    /// Check if a username is already taken
    fn email_exists(&self, username: &str) -> bool;

    /// Deposits the given amount into the account with the given ID.
    /// # Errors
    /// Returns an error if the account does not exist.
    // TODO: is this necessary?
    fn deposit_to_account(
        &mut self,
        account_id: &AccountId,
        amount: f64,
    ) -> Result<(), &'static str>;

    /// Withdraws the given amount from the account with the given ID.
    /// # Errors
    /// Returns an error if the account does not exist or if there are insufficient funds.
    fn withdraw_from_account(
        &mut self,
        account_id: &AccountId,
        amount: f64,
    ) -> Result<(), &'static str>;

    /// Gets the balance of the account with the given ID.
    fn get_balance(&self, account_id: &AccountId) -> Option<f64>;
}

impl AccountRepoExt for AccountRepo {
    fn create_account(
        &mut self,
        firstname: String,
        surname: String,
        email: String,
        password: String,
        initial_balance: f64,
    ) -> AccountId {
        let id = Uuid::new_v4();
        let account = Account::new(email, firstname, surname, password, initial_balance);
        self.insert(id, account);
        id
    }

    fn authenticate(&self, email: &str, password: &str) -> Option<AccountId> {
        // Find account by username and verify password
        for (id, account) in self.iter() {
            if account.email == email && account.verify_password(password) {
                return Some(*id);
            }
        }
        None
    }

    fn email_exists(&self, email: &str) -> bool {
        self.iter().any(|(_, account)| account.email == email)
    }

    fn get_account_id_by_email(&self, email: &str) -> Option<AccountId> {
        for (id, account) in self.iter() {
            if account.email == email {
                return Some(*id);
            }
        }
        None
    }

    fn deposit_to_account(
        &mut self,
        account_id: &AccountId,
        amount: f64,
    ) -> Result<(), &'static str> {
        if let Some(account) = self.get_mut(account_id) {
            account.deposit(amount);
            Ok(())
        } else {
            Err("Account not found")
        }
    }

    fn withdraw_from_account(
        &mut self,
        account_id: &AccountId,
        amount: f64,
    ) -> Result<(), &'static str> {
        if let Some(account) = self.get_mut(account_id) {
            account.withdraw(amount).map_err(|_| "Insufficient funds")
        } else {
            Err("Account not found")
        }
    }

    fn get_balance(&self, account_id: &AccountId) -> Option<f64> {
        self.get(account_id).map(|account| account.balance)
    }
}
