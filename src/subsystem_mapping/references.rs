use std::marker::PhantomData;

#[derive(Debug)]
pub struct ReferenceByIndex<T> {
    id: String,
    index: Option<usize>,
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
}
