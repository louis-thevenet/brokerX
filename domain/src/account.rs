use color_eyre::Result;
use in_memory_adapter::InMemoryRepo;

#[derive(Debug)]
pub struct Account {
    pub name: String,
    pub email: String,
    pub balance: f64,
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

    fn deposit(&mut self, amount: f64) {
        self.balance += amount;
    }

    fn withdraw(&mut self, amount: f64) -> Result<(), NotEnoughMoneyError> {
        if self.balance < amount {
            return Err(NotEnoughMoneyError);
        }
        self.balance -= amount;
        Ok(())
    }
}

pub type AccountId = u32;

pub type AccountRepo = InMemoryRepo<Account, AccountId>;
