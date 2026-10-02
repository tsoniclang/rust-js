use super::{JsClosedValue, JsClosedValueCarrier, JsClosedValuePayload, JsValue};
use crate::errors::{unsupported, JsResult};
use std::fmt;
use std::rc::Rc;
use tsonic_rust_runtime::{EmptyObject, ObjectIdentityCarrier};

struct NativeValue<Value> {
    _value: Value,
}

impl<Value> fmt::Debug for NativeValue<Value> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("NativeValue")
    }
}

impl<Value: 'static> JsClosedValueCarrier for NativeValue<Value> {
    fn identity_key(&self) -> usize {
        (self as *const Self).addr()
    }

    fn inspect_value(&self) -> String {
        "[Native value]".to_owned()
    }

    fn write_string(&self, _output: &mut String) -> JsResult<()> {
        Err(unsupported(
            "Native value erasure does not expose string conversion",
        ))
    }

    fn project_json(&self) -> JsResult<JsValue> {
        Err(unsupported(
            "Native value erasure does not expose a JSON projection",
        ))
    }
}

impl JsValue {
    pub fn from_shared_identity(value: Rc<dyn ObjectIdentityCarrier>) -> Self {
        Self::Closed(JsClosedValue(JsClosedValuePayload::NativeShared(value)))
    }

    pub fn from_closed<Value: 'static>(value: Value) -> Self {
        Self::closed(NativeValue { _value: value })
    }
}

impl From<EmptyObject> for JsValue {
    fn from(value: EmptyObject) -> Self {
        Self::Closed(JsClosedValue(JsClosedValuePayload::Empty(value)))
    }
}
