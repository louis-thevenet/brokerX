use std::{collections::HashMap, hash::Hash};

#[derive(Debug, Default)]
pub struct InMemoryRepo<T, Id> {
    storage: HashMap<Id, T>,
}

impl<T, Id> InMemoryRepo<T, Id>
where
    Id: Clone + Eq + Hash,
{
    #[must_use]
    pub fn new() -> Self {
        Self {
            storage: HashMap::new(),
        }
    }

    pub fn insert(&mut self, id: Id, item: T) {
        self.storage.insert(id, item);
    }

    pub fn get(&self, id: &Id) -> Option<&T> {
        self.storage.get(id)
    }
}
