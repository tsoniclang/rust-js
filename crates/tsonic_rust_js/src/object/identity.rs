use crate::equality::{hash_identity, JsHash, JsSameValue, JsSameValueZero, JsStrictEqual};
use tsonic_rust_runtime::{EmptyObject, ObjectIdentityCarrier};

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
        hash_identity(self.object_identity().key())
    }
}
