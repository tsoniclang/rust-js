//! Closed JS runtime value carrier.

use std::cell::RefCell;
use std::collections::HashSet;
use std::fmt;
use std::rc::Rc;

use crate::array::JsArray;
use crate::equality::{same_value_zero_f64, strict_equal_f64, JsSameValueZero, JsStrictEqual};
use crate::object::JsObject;

#[derive(Clone, Debug)]
pub enum JsValue {
    Undefined,
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Object(Rc<RefCell<JsObject>>),
    Array(JsArray<JsValue>),
}

impl JsValue {
    pub const fn undefined() -> Self {
        Self::Undefined
    }

    pub const fn null() -> Self {
        Self::Null
    }

    /// Wraps an object payload in a fresh reference-identity handle.
    pub fn object(object: JsObject) -> Self {
        Self::Object(Rc::new(RefCell::new(object)))
    }

    /// Wraps an array reference-identity handle.
    pub fn array(values: JsArray<JsValue>) -> Self {
        Self::Array(values)
    }

    /// Returns the object handle when the value is an object.
    pub fn as_object(&self) -> Option<&Rc<RefCell<JsObject>>> {
        match self {
            Self::Object(object) => Some(object),
            _ => None,
        }
    }

    /// Returns the array handle when the value is an array.
    pub fn as_array(&self) -> Option<&JsArray<JsValue>> {
        match self {
            Self::Array(values) => Some(values),
            _ => None,
        }
    }

    pub fn is_nullish(&self) -> bool {
        matches!(self, Self::Undefined | Self::Null)
    }

    pub fn inspect(&self) -> String {
        self.inspect_with_limits(Some(2), Some(100))
    }

    pub fn inspect_with_limits(
        &self,
        max_depth: Option<usize>,
        max_entries: Option<usize>,
    ) -> String {
        InspectState {
            active: HashSet::new(),
            max_depth: max_depth.unwrap_or(MAX_INSPECT_DEPTH),
            max_entries: max_entries.unwrap_or(usize::MAX),
        }
        .render(self, 0)
    }
}

const MAX_INSPECT_DEPTH: usize = 64;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum ContainerId {
    Object(usize),
    Array(usize),
}

struct InspectState {
    active: HashSet<ContainerId>,
    max_depth: usize,
    max_entries: usize,
}

impl InspectState {
    fn render(&mut self, value: &JsValue, depth: usize) -> String {
        match value {
            JsValue::Undefined => "undefined".to_string(),
            JsValue::Null => "null".to_string(),
            JsValue::Bool(value) => value.to_string(),
            JsValue::Number(value) => format_js_number(*value),
            JsValue::String(value) => format!("{value:?}"),
            JsValue::Object(object) => self.render_object(object, depth),
            JsValue::Array(values) => self.render_array(values, depth),
        }
    }

    fn render_object(&mut self, object: &Rc<RefCell<JsObject>>, depth: usize) -> String {
        if depth > self.max_depth {
            return "[Object]".to_string();
        }
        let id = ContainerId::Object(Rc::as_ptr(object) as usize);
        if !self.active.insert(id) {
            return "[Circular]".to_string();
        }
        let entries = match object.try_borrow() {
            Ok(object) => object.entries(),
            Err(_) => {
                self.active.remove(&id);
                return "[Uninspectable]".to_string();
            }
        };
        let total = entries.len();
        let mut rendered = entries
            .into_iter()
            .take(self.max_entries)
            .map(|(key, value)| format!("{key}: {}", self.render(&value, depth + 1)))
            .collect::<Vec<_>>();
        append_remaining(&mut rendered, total, self.max_entries);
        self.active.remove(&id);
        format!("{{{}}}", rendered.join(", "))
    }

    fn render_array(&mut self, values: &JsArray<JsValue>, depth: usize) -> String {
        if depth > self.max_depth {
            return "[Array]".to_string();
        }
        let id = ContainerId::Array(values.identity());
        if !self.active.insert(id) {
            return "[Circular]".to_string();
        }
        let values = values.values();
        let total = values.len();
        let mut rendered = values
            .into_iter()
            .take(self.max_entries)
            .map(|value| {
                value
                    .map(|value| self.render(&value, depth + 1))
                    .unwrap_or_default()
            })
            .collect::<Vec<_>>();
        append_remaining(&mut rendered, total, self.max_entries);
        self.active.remove(&id);
        format!("[{}]", rendered.join(", "))
    }
}

fn append_remaining(rendered: &mut Vec<String>, total: usize, max_entries: usize) {
    if total > max_entries {
        rendered.push(format!("... {} more items", total - max_entries));
    }
}

impl PartialEq for JsValue {
    fn eq(&self, other: &Self) -> bool {
        self.strict_equal(other)
    }
}

impl Eq for JsValue {}

impl JsSameValueZero for JsValue {
    fn same_value_zero(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Undefined, Self::Undefined) | (Self::Null, Self::Null) => true,
            (Self::Bool(left), Self::Bool(right)) => left == right,
            (Self::Number(left), Self::Number(right)) => same_value_zero_f64(*left, *right),
            (Self::String(left), Self::String(right)) => left == right,
            (Self::Object(left), Self::Object(right)) => Rc::ptr_eq(left, right),
            (Self::Array(left), Self::Array(right)) => left.ptr_eq(right),
            _ => false,
        }
    }
}

impl JsStrictEqual for JsValue {
    fn strict_equal(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Undefined, Self::Undefined) | (Self::Null, Self::Null) => true,
            (Self::Bool(left), Self::Bool(right)) => left == right,
            (Self::Number(left), Self::Number(right)) => strict_equal_f64(*left, *right),
            (Self::String(left), Self::String(right)) => left == right,
            (Self::Object(left), Self::Object(right)) => Rc::ptr_eq(left, right),
            (Self::Array(left), Self::Array(right)) => left.ptr_eq(right),
            _ => false,
        }
    }
}

impl From<Vec<JsValue>> for JsValue {
    fn from(values: Vec<JsValue>) -> Self {
        Self::array(JsArray::from_dense(values))
    }
}

impl From<bool> for JsValue {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<f64> for JsValue {
    fn from(value: f64) -> Self {
        Self::Number(value)
    }
}

impl From<i32> for JsValue {
    fn from(value: i32) -> Self {
        Self::Number(f64::from(value))
    }
}

impl From<String> for JsValue {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

pub fn from_string(value: &str) -> JsValue {
    JsValue::String(value.to_owned())
}

pub fn clone_value(value: &JsValue) -> JsValue {
    value.clone()
}

impl fmt::Display for JsValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.inspect())
    }
}

fn format_js_number(value: f64) -> String {
    if value.is_nan() {
        return "NaN".to_string();
    }
    if value == f64::INFINITY {
        return "Infinity".to_string();
    }
    if value == f64::NEG_INFINITY {
        return "-Infinity".to_string();
    }
    if value == 0.0 && value.is_sign_negative() {
        return "-0".to_string();
    }
    let mut text = value.to_string();
    if text.ends_with(".0") {
        text.truncate(text.len() - 2);
    }
    text
}
