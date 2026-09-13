use crate::equality::{hash_identity, JsHash, JsSameValue, JsSameValueZero, JsStrictEqual};
use std::cell::Cell;
use std::rc::Rc;
use tsonic_rust_runtime::{ObjectIdentity, ObjectIdentityCarrier};

#[derive(Clone, Debug)]
pub struct EmptyObject {
    identity: ObjectIdentity,
    frozen: Rc<Cell<bool>>,
}

impl EmptyObject {
    pub fn new() -> Self {
        Self {
            identity: ObjectIdentity::new(),
            frozen: Rc::new(Cell::new(false)),
        }
    }

    pub fn freeze(&self) -> Self {
        self.frozen.set(true);
        self.clone()
    }

    pub fn is_frozen(&self) -> bool {
        self.frozen.get()
    }
}

impl Default for EmptyObject {
    fn default() -> Self {
        Self::new()
    }
}

impl ObjectIdentityCarrier for EmptyObject {
    fn object_identity(&self) -> &ObjectIdentity {
        &self.identity
    }
}

impl PartialEq for EmptyObject {
    fn eq(&self, other: &Self) -> bool {
        self.identity == other.identity
    }
}

impl Eq for EmptyObject {}

impl JsSameValueZero for EmptyObject {
    fn same_value_zero(&self, other: &Self) -> bool {
        self == other
    }
}

impl JsSameValue for EmptyObject {
    fn same_value(&self, other: &Self) -> bool {
        self == other
    }
}

impl JsStrictEqual for EmptyObject {
    fn strict_equal(&self, other: &Self) -> bool {
        self == other
    }
}

impl JsHash for EmptyObject {
    fn js_hash(&self) -> u64 {
        hash_identity(self.identity.key())
    }
}
