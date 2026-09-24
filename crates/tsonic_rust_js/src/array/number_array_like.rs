use super::JsArray;
use crate::typed_array::{TypedArray, TypedElement};

pub trait NumberArrayLike {
    fn number_array_length(&self) -> usize;
    fn number_array_get(&self, index: f64) -> Option<f64>;
    fn number_array_copy(&self) -> JsArray<f64>;
}

impl NumberArrayLike for JsArray<f64> {
    fn number_array_length(&self) -> usize {
        self.len()
    }

    fn number_array_get(&self, index: f64) -> Option<f64> {
        self.get_number(index)
    }

    fn number_array_copy(&self) -> JsArray<f64> {
        JsArray::from_values(self.iter_values())
    }
}

impl<Element: TypedElement> NumberArrayLike for TypedArray<Element>
where
    Element::Value: Into<f64>,
{
    fn number_array_length(&self) -> usize {
        self.length()
    }

    fn number_array_get(&self, index: f64) -> Option<f64> {
        self.get_number(index).map(Into::into)
    }

    fn number_array_copy(&self) -> JsArray<f64> {
        JsArray::from_values((0..self.len()).map(|index| {
            self.get_usize(index)
                .expect("checked typed-array index invariant violated")
                .into_value()
                .into()
        }))
    }
}

pub fn number_array_length<Value: NumberArrayLike>(value: &Value) -> usize {
    value.number_array_length()
}

pub fn number_array_get<Value: NumberArrayLike>(value: &Value, index: f64) -> Option<f64> {
    value.number_array_get(index)
}

pub fn number_array_from<Value: NumberArrayLike>(value: &Value) -> JsArray<f64> {
    value.number_array_copy()
}
