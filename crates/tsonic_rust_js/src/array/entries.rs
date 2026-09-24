use std::cell::RefCell;
use std::iter::FusedIterator;
use std::rc::Rc;

use super::JsArray;
use tsonic_rust_runtime::{IteratorResult, ObjectIdentity, ObjectIdentityCarrier};

struct EntriesState<T> {
    array: Option<JsArray<T>>,
    index: usize,
}

pub struct JsArrayEntries<T> {
    state: Rc<RefCell<EntriesState<T>>>,
    identity: ObjectIdentity,
}

impl<T> JsArrayEntries<T> {
    pub(super) fn new(array: JsArray<T>) -> Self {
        Self {
            state: Rc::new(RefCell::new(EntriesState {
                array: Some(array),
                index: 0,
            })),
            identity: ObjectIdentity::new(),
        }
    }
}

impl<T: Clone> JsArrayEntries<T> {
    pub fn next_result(&self) -> IteratorResult<(usize, T), ()> {
        match self.clone().next() {
            Some(value) => IteratorResult::yielded(value),
            None => IteratorResult::completed(()),
        }
    }
}

impl<T> Clone for JsArrayEntries<T> {
    fn clone(&self) -> Self {
        Self {
            state: Rc::clone(&self.state),
            identity: self.identity.clone(),
        }
    }
}

impl<T> ObjectIdentityCarrier for JsArrayEntries<T> {
    fn object_identity(&self) -> &ObjectIdentity {
        &self.identity
    }
}

impl<T: Clone> Iterator for JsArrayEntries<T> {
    type Item = (usize, T);

    fn next(&mut self) -> Option<Self::Item> {
        let mut state = self.state.borrow_mut();
        let array = state.array.as_ref()?;
        if state.index >= array.len() {
            state.array = None;
            return None;
        }
        let value = array.get(state.index)?;
        let index = state.index;
        state.index += 1;
        Some((index, value))
    }
}

impl<T: Clone> FusedIterator for JsArrayEntries<T> {}
