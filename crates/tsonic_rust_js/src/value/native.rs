use super::{JsClosedValue, JsClosedValuePayload, JsValue};
use std::rc::Rc;
use tsonic_rust_runtime::{EmptyObject, NativePayload, ObjectIdentityCarrier};

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
            Self::Closed(JsClosedValue(JsClosedValuePayload::Native(value))) => {
                value.native_value()
            }
            _ => None,
        }
    }

    pub fn from_shared_identity(value: Rc<dyn ObjectIdentityCarrier>) -> Self {
        Self::Closed(JsClosedValue(JsClosedValuePayload::NativeShared(value)))
    }

    pub fn from_closed<Value: 'static>(value: Value) -> Self {
        Self::Closed(JsClosedValue(JsClosedValuePayload::Native(
            NativePayload::from_closed(value),
        )))
    }
}

impl From<EmptyObject> for JsValue {
    fn from(value: EmptyObject) -> Self {
        Self::Closed(JsClosedValue(JsClosedValuePayload::Empty(value)))
    }
}
