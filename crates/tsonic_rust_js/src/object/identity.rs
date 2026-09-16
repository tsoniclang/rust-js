use crate::equality::{hash_identity, JsHash, JsSameValue, JsSameValueZero, JsStrictEqual};
use crate::errors::JsResult;
use crate::object::JsObject;
use crate::value::{JsClosedValueCarrier, JsValue};
use tsonic_rust_runtime::{EmptyObject, ObjectIdentity, ObjectIdentityCarrier};

impl JsClosedValueCarrier for EmptyObject {
    fn identity_key(&self) -> usize {
        self.object_identity().key()
    }

    fn inspect_value(&self) -> String {
        "[object Object]".to_owned()
    }

    fn project_json(&self) -> JsResult<JsValue> {
        Ok(JsValue::object(JsObject::new()))
    }
}

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

impl JsSameValueZero for ObjectIdentity {
    fn same_value_zero(&self, other: &Self) -> bool {
        self == other
    }
}

impl JsSameValue for ObjectIdentity {
    fn same_value(&self, other: &Self) -> bool {
        self == other
    }
}

impl JsStrictEqual for ObjectIdentity {
    fn strict_equal(&self, other: &Self) -> bool {
        self == other
    }
}

impl JsHash for ObjectIdentity {
    fn js_hash(&self) -> u64 {
        hash_identity(self.key())
    }
}
