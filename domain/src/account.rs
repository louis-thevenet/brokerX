use color_eyre::Result;
use in_memory_adapter::InMemoryRepo;
use uuid::Uuid;

#[derive(Debug)]
pub struct Account {
    pub name: String,
    pub email: String,
    balance: f64,
}
pub struct NotEnoughMoneyError;
impl Account {
    #[must_use]
    pub fn new(name: String, email: String, balance: f64) -> Self {
        Self {
            name,
            email,
            balance,
        }
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
    /// Creates a new account with the given name, email, and initial balance.
    fn create_account(&mut self, name: String, email: String, initial_balance: f64) -> AccountId;

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
    fn create_account(&mut self, name: String, email: String, initial_balance: f64) -> AccountId {
        let id = Uuid::new_v4();
        let account = Account::new(name, email, initial_balance);
        self.insert(id, account);
        id
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
