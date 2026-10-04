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
    fn native_value(&self) -> Option<&dyn core::any::Any> {
        Some(&self._value)
    }

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
    pub fn native_shared<Payload: ?Sized + 'static>(&self) -> Option<Rc<Payload>> {
        match self {
            Self::Closed(JsClosedValue(JsClosedValuePayload::NativeShared(value))) => {
                let mut selected = None;
                Rc::clone(value).project_native(&mut selected);
                selected
            }
            _ => None,
        }
    }

    pub fn native_value<Payload: Clone + 'static>(&self) -> Option<Payload> {
        match self {
            Self::Closed(JsClosedValue(JsClosedValuePayload::Object(value))) => {
                value.native_value()?.downcast_ref::<Payload>().cloned()
            }
            _ => None,
        }
    }

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
