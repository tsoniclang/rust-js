use super::{JsArray, JsArrayOwner};
use crate::equality::{hash_identity, JsHash, JsSameValueZero, JsStrictEqual};
use crate::numeric::IndexInput;
use crate::string::JsToString;
use crate::value::JsValue;
use std::any::Any;
use std::fmt;
use std::rc::Rc;

pub enum JsArrayElement<'value> {
    Value(&'value JsValue),
    String(&'value str),
}

impl JsArrayElement<'_> {
    pub fn to_value(&self) -> JsValue {
        match self {
            Self::Value(value) => (*value).clone(),
            Self::String(value) => JsValue::String((*value).to_owned()),
        }
    }
}

#[derive(Clone, Copy)]
pub enum JsArrayIndex<'key> {
    Dense(usize),
    Property(&'key str),
}

pub type JsArrayProjection =
    fn(&JsArrayValue, JsArrayIndex<'_>, &mut dyn FnMut(JsArrayElement<'_>));

trait NativeArrayStorage: Any + tsonic_rust_runtime::ObjectIdentityCarrier {
    fn len(&self) -> usize;
    fn truncate(&self, length: usize);
}

impl<Value> tsonic_rust_runtime::ObjectIdentityCarrier for JsArrayOwner<Value> {
    fn object_identity(&self) -> &tsonic_rust_runtime::ObjectIdentity {
        self.identity
            .get_or_init(tsonic_rust_runtime::ObjectIdentity::new)
    }
}

impl<Value: 'static> NativeArrayStorage for JsArrayOwner<Value> {
    fn len(&self) -> usize {
        self.values.borrow().values.len()
    }

    fn truncate(&self, length: usize) {
        self.values.borrow_mut().truncate(length);
    }
}

#[derive(Clone)]
pub struct JsArrayValue {
    owner: Rc<dyn NativeArrayStorage>,
    project: JsArrayProjection,
}

impl fmt::Debug for JsArrayValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("JsArrayValue")
            .field(&self.len())
            .finish()
    }
}

impl JsHash for JsArrayValue {
    fn js_hash(&self) -> u64 {
        hash_identity(self.identity())
    }
}

impl Default for JsArrayValue {
    fn default() -> Self {
        Self::new(&JsArray::<JsValue>::new(), project_broad_array_element)
    }
}

impl PartialEq for JsArrayValue {
    fn eq(&self, other: &Self) -> bool {
        self.ptr_eq(other)
    }
}

impl Eq for JsArrayValue {}

impl JsStrictEqual for JsArrayValue {
    fn strict_equal(&self, other: &Self) -> bool {
        self.ptr_eq(other)
    }
}

impl JsSameValueZero for JsArrayValue {
    fn same_value_zero(&self, other: &Self) -> bool {
        self.ptr_eq(other)
    }
}

impl tsonic_rust_runtime::ObjectIdentityCarrier for JsArrayValue {
    fn object_identity(&self) -> &tsonic_rust_runtime::ObjectIdentity {
        self.owner.object_identity()
    }
}

impl JsArrayValue {
    pub fn new<Value: 'static>(values: &JsArray<Value>, project: JsArrayProjection) -> Self {
        Self {
            owner: values.state.clone(),
            project,
        }
    }

    pub fn restore<Value: 'static>(&self) -> Option<JsArray<Value>> {
        let owner: Rc<dyn Any> = self.owner.clone();
        owner
            .downcast::<JsArrayOwner<Value>>()
            .ok()
            .map(|state| JsArray { state })
    }

    pub fn cast<Value: 'static>(&self) -> crate::errors::JsResult<JsArray<Value>> {
        self.restore().ok_or_else(|| {
            crate::errors::type_error(
                "An array assertion requires the exact native element backing.",
            )
        })
    }

    pub fn with_native_element<Value: 'static, Output>(
        &self,
        index: JsArrayIndex<'_>,
        read: impl FnOnce(Option<&Value>) -> Output,
    ) -> Output {
        let state = self.native_owner::<Value>().values.borrow();
        read(match index {
            JsArrayIndex::Dense(index) => state.values.get(index),
            JsArrayIndex::Property(key) => state
                .numeric_properties
                .iter()
                .find_map(|(candidate, value)| (candidate == key).then_some(value)),
        })
    }

    fn native_owner<Value: 'static>(&self) -> &JsArrayOwner<Value> {
        self.checked_native_owner()
            .expect("An array operation requires its statically selected native backing.")
    }

    fn checked_native_owner<Value: 'static>(
        &self,
    ) -> crate::errors::JsResult<&JsArrayOwner<Value>> {
        let owner: &dyn Any = self.owner.as_ref();
        owner.downcast_ref::<JsArrayOwner<Value>>().ok_or_else(|| {
            crate::errors::type_error(
                "An array operation requires the exact native element backing.",
            )
        })
    }

    pub fn get_native_element<Value: Clone + 'static>(
        &self,
        index: JsArrayIndex<'_>,
    ) -> Option<Value> {
        self.with_native_element(index, |value: Option<&Value>| value.cloned())
    }

    pub fn identity(&self) -> usize {
        Rc::as_ptr(&self.owner).cast::<()>().addr()
    }

    pub fn ptr_eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.owner, &other.owner)
    }

    pub fn len(&self) -> usize {
        self.owner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn has_index(&self, index: usize) -> bool {
        index < self.len()
    }

    pub fn set_len(&self, length: impl IndexInput) {
        self.owner.truncate(
            length
                .checked_integer()
                .expect("Array length must be an exact native index"),
        );
    }

    pub fn visit_element(&self, index: usize, visitor: &mut dyn FnMut(JsArrayElement<'_>)) {
        if !self.has_index(index) {
            visitor(JsArrayElement::Value(&JsValue::Null));
            return;
        }
        (self.project)(self, JsArrayIndex::Dense(index), visitor);
    }

    pub fn get(&self, index: usize) -> Option<JsValue> {
        if !self.has_index(index) {
            return None;
        }
        let mut result = None;
        self.visit_element(index, &mut |value| result = Some(value.to_value()));
        result
    }

    pub fn get_number(&self, index: impl IndexInput + JsToString) -> Option<JsValue> {
        if let Some(index) = super::canonical_array_index(index) {
            return self.get(index);
        }
        let key = index.to_js_string();
        let mut result = None;
        (self.project)(self, JsArrayIndex::Property(&key), &mut |value| {
            result = Some(value.to_value())
        });
        result
    }

    pub fn set(&self, index: usize, value: JsValue) {
        self.native_owner::<JsValue>()
            .values
            .borrow_mut()
            .set(index, value);
    }

    pub fn set_number(
        &self,
        index: impl IndexInput + JsToString,
        value: JsValue,
    ) -> crate::errors::JsResult<()> {
        let owner = self.checked_native_owner::<JsValue>()?;
        if let Some(index) = super::canonical_array_index(index) {
            let mut state = owner.values.borrow_mut();
            if index > state.values.len() {
                return Err(crate::errors::range_error(
                    "Array assignment exceeds initialized dense storage",
                ));
            }
            state.set(index, value);
        } else {
            owner
                .values
                .borrow_mut()
                .set_property(index.to_js_string(), value);
        }
        Ok(())
    }

    pub fn push(&self, value: JsValue) -> usize {
        self.native_owner::<JsValue>()
            .values
            .borrow_mut()
            .push(value)
    }

    pub fn values(&self) -> Vec<JsValue> {
        self.iter_values().collect()
    }

    pub fn entries(&self) -> impl Iterator<Item = (usize, JsValue)> {
        self.iter_values().enumerate()
    }

    pub fn iter_values(&self) -> JsArrayValueIterator {
        JsArrayValueIterator {
            array: self.clone(),
            index: 0,
        }
    }
}

pub struct JsArrayValueIterator {
    array: JsArrayValue,
    index: usize,
}

impl Iterator for JsArrayValueIterator {
    type Item = JsValue;

    fn next(&mut self) -> Option<Self::Item> {
        let value = self.array.get(self.index)?;
        self.index += 1;
        Some(value)
    }
}

pub fn project_broad_array_element(
    array: &JsArrayValue,
    index: JsArrayIndex<'_>,
    visitor: &mut dyn FnMut(JsArrayElement<'_>),
) {
    array.with_native_element::<JsValue, _>(index, |value| {
        if let Some(value) = value {
            visitor(JsArrayElement::Value(value));
        }
    });
}
