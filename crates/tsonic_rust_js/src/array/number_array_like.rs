use super::JsArray;
use crate::typed_array::{TypedArray, TypedElement};

pub trait NumberArrayLike {
    fn number_array_length(&self) -> f64;
    fn number_array_get(&self, index: f64) -> Option<f64>;
}

impl NumberArrayLike for JsArray<f64> {
    fn number_array_length(&self) -> f64 {
        self.len() as f64
    }

    fn number_array_get(&self, index: f64) -> Option<f64> {
        self.get_number(index)
    }
}

impl<Element: TypedElement> NumberArrayLike for TypedArray<Element> {
    fn number_array_length(&self) -> f64 {
        self.length()
    }

    fn number_array_get(&self, index: f64) -> Option<f64> {
        self.get_number(index)
    }
}

pub fn number_array_length<Value: NumberArrayLike>(value: &Value) -> f64 {
    value.number_array_length()
}

pub fn number_array_get<Value: NumberArrayLike>(value: &Value, index: f64) -> Option<f64> {
    value.number_array_get(index)
}

pub fn number_array_from<Value: NumberArrayLike>(value: &Value) -> JsArray<f64> {
    JsArray::from_values((0..value.number_array_length() as usize).map(|index| {
        value
            .number_array_get(index as f64)
            .expect("checked array density invariant violated")
    }))
}
