use serde_derive::Serialize;
use std::collections::HashMap;
use std::marker::PhantomData;

#[derive(Debug, Serialize)]
pub struct ReferenceByIndex<T> {
    id: String,
    index: Option<usize>,

    #[serde(skip_serializing)]
    phantom: PhantomData<T>,
}

impl<T> ReferenceByIndex<T> {
    pub fn new(id: &String) -> ReferenceByIndex<T> {
        ReferenceByIndex {
            id: id.clone(),
            index: None,
            phantom: PhantomData,
        }
    }

    pub fn find_index_in(&mut self, indexes: &HashMap<String, usize>) {
        self.index = indexes.get(&self.id).map(|i| *i);
    }

    pub fn index(&self) -> Option<usize> {
        self.index
    }
}
