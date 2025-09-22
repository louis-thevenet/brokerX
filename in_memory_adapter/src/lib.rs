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

    pub fn update(&mut self, id: Id, item: T) {
        self.storage.insert(id, item);
    }

    pub fn remove(&mut self, id: &Id) -> Option<T> {
        self.storage.remove(id)
    }

    pub fn get(&self, id: &Id) -> Option<&T> {
        self.storage.get(id)
    }

    pub fn get_mut(&mut self, id: &Id) -> Option<&mut T> {
        self.storage.get_mut(id)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.storage.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.storage.is_empty()
    }

    /// Iterate over all (id, item) pairs in the repository
    pub fn iter(&self) -> impl Iterator<Item = (&Id, &T)> {
        self.storage.iter()
    }
}
