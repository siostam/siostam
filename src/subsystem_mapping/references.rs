use serde_derive::Serialize;
use std::collections::HashMap;
use std::marker::PhantomData;

/// A ReferenceByIndex is used in the graph representation to store
/// a link between systems/subsystems. It is stored using an immutable index to get easy
/// processing in JSON.
///
/// For example, if this is a ReferenceByIndex<System>, you can access the system
/// by simply doing `graph.systems[ref.index]`.
#[derive(Debug, Serialize)]
pub struct ReferenceByIndex<T> {
    id: String,
    index: Option<usize>,

    /// The phantom data is only there to keet track of the type
    #[serde(skip_serializing)]
    phantom: PhantomData<T>,
}

impl<T> ReferenceByIndex<T> {
    /// The reference does not store the index right away because we have to wait for all systems
    /// and subsystems to be there. Instead, we reconstruct the link using find_index_in later.
    pub fn new(id: &String) -> ReferenceByIndex<T> {
        ReferenceByIndex {
            id: id.clone(),
            index: None,
            phantom: PhantomData,
        }
    }

    /// Use this to set the index when the items are all gathered in a HashMap
    pub fn find_index_in(&mut self, indexes: &HashMap<String, usize>) {
        self.index = indexes.get(&self.id).map(|i| *i);
    }

    /// Simple getter for the index. May be None if the referenced item is missing
    pub fn index(&self) -> Option<usize> {
        self.index
    }
}
