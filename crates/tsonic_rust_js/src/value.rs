//! Closed JS runtime value carrier.

use std::cell::RefCell;
use std::collections::HashSet;
use std::fmt;
use std::rc::Rc;

use crate::array::JsArray;
use crate::equality::{
    hash_identity, same_value_f64, same_value_zero_f64, strict_equal_f64, JsHash, JsSameValue,
    JsSameValueZero, JsStrictEqual,
};
use crate::errors::JsResult;
use crate::object::JsObject;
use crate::{JsString, JsSymbol};
use tsonic_rust_runtime::{Null, Undefined};

pub trait JsClosedValueCarrier: fmt::Debug {
    fn identity_key(&self) -> usize;
    fn inspect_value(&self) -> String;
    fn project_json(&self) -> JsResult<JsValue>;
}

#[derive(Clone)]
pub struct JsClosedValue(Rc<dyn JsClosedValueCarrier>);

impl fmt::Debug for JsClosedValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.inspect())
    }
}

impl JsClosedValue {
    pub fn new<T>(value: T) -> Self
    where
        T: JsClosedValueCarrier + 'static,
    {
        Self(Rc::new(value))
    }

    pub fn identity_key(&self) -> usize {
        self.0.identity_key()
    }

    pub fn inspect(&self) -> String {
        self.0.inspect_value()
    }

    pub fn project_json(&self) -> JsResult<JsValue> {
        self.0.project_json()
    }
}

#[derive(Clone)]
pub struct JsonProjection(Rc<JsonProjectionInner>);

struct JsonProjectionInner {
    project: Box<dyn Fn(String) -> JsResult<JsValue>>,
}

impl fmt::Debug for JsonProjection {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("JsonProjection")
    }
}

impl JsonProjection {
    pub fn new<T, F>(source: T, project: F) -> Self
    where
        T: 'static,
        F: Fn(&T, String) -> JsResult<JsValue> + 'static,
    {
        Self(Rc::new(JsonProjectionInner {
            project: Box::new(move |key| project(&source, key)),
        }))
    }

    pub fn project(&self, key: String) -> JsResult<JsValue> {
        (self.0.project)(key)
    }

    fn ptr_eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }

    pub(crate) fn identity(&self) -> usize {
        Rc::as_ptr(&self.0) as usize
    }
}

#[derive(Clone, Debug)]
pub enum JsValue {
    Undefined,
    Null,
    Bool(bool),
    Number(f64),
    String(JsString),
    Symbol(JsSymbol),
    Object(Rc<RefCell<JsObject>>),
    Array(JsArray<JsValue>),
    Closed(JsClosedValue),
    JsonProjection(JsonProjection),
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

    pub fn symbol(value: JsSymbol) -> Self {
        Self::Symbol(value)
    }

    pub fn closed<T>(value: T) -> Self
    where
        T: JsClosedValueCarrier + 'static,
    {
        Self::Closed(JsClosedValue::new(value))
    }

    pub fn as_symbol(&self) -> Option<&JsSymbol> {
        match self {
            Self::Symbol(value) => Some(value),
            _ => None,
        }
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

    pub fn reference_identity_key(&self) -> Option<usize> {
        match self {
            Self::Object(value) => Some(Rc::as_ptr(value) as usize),
            Self::Array(value) => Some(value.identity()),
            Self::Closed(value) => Some(value.identity_key()),
            Self::JsonProjection(value) => Some(value.identity()),
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

pub fn js_value_from_array<T, F>(values: &JsArray<T>, mut convert: F) -> JsValue
where
    T: Clone,
    F: FnMut(T) -> JsValue,
{
    let length = values.len();
    let converted = values
        .entries()
        .into_iter()
        .filter_map(|(index, value)| value.map(|value| (index, convert(value))))
        .collect();
    JsValue::array(JsArray::from_sparse(length, converted))
}

pub fn js_value_from_optional_pairs<K>(pairs: Vec<Option<(K, JsValue)>>) -> JsValue
where
    K: AsRef<str>,
{
    JsValue::object(JsObject::from_pairs(pairs.into_iter().flatten()))
}

pub fn js_value_from_json_projection<T, F>(source: T, project: F) -> JsValue
where
    T: 'static,
    F: Fn(&T, String) -> JsResult<JsValue> + 'static,
{
    JsValue::JsonProjection(JsonProjection::new(source, project))
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
            JsValue::String(value) => value.inspect_quoted(),
            JsValue::Symbol(value) => format!("{value:?}"),
            JsValue::Object(object) => self.render_object(object, depth),
            JsValue::Array(values) => self.render_array(values, depth),
            JsValue::Closed(value) => value.inspect(),
            JsValue::JsonProjection(_) => "[JSON projection]".to_string(),
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
            Ok(object) => object.entries_exact(),
            Err(_) => {
                self.active.remove(&id);
                return "[Uninspectable]".to_string();
            }
        };
        let total = entries.len();
        let mut rendered = entries
            .into_iter()
            .take(self.max_entries)
            .map(|(key, value)| {
                format!(
                    "{}: {}",
                    key.to_utf8_escaped(),
                    self.render(&value, depth + 1)
                )
            })
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

impl JsSameValue for JsValue {
    fn same_value(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Undefined, Self::Undefined) | (Self::Null, Self::Null) => true,
            (Self::Bool(left), Self::Bool(right)) => left == right,
            (Self::Number(left), Self::Number(right)) => same_value_f64(*left, *right),
            (Self::String(left), Self::String(right)) => left == right,
            (Self::Symbol(left), Self::Symbol(right)) => left == right,
            (Self::Object(left), Self::Object(right)) => Rc::ptr_eq(left, right),
            (Self::Array(left), Self::Array(right)) => left.ptr_eq(right),
            (Self::Closed(left), Self::Closed(right)) => {
                left.identity_key() == right.identity_key()
            }
            (Self::JsonProjection(left), Self::JsonProjection(right)) => left.ptr_eq(right),
            _ => false,
        }
    }
}

impl JsSameValueZero for JsValue {
    fn same_value_zero(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Undefined, Self::Undefined) | (Self::Null, Self::Null) => true,
            (Self::Bool(left), Self::Bool(right)) => left == right,
            (Self::Number(left), Self::Number(right)) => same_value_zero_f64(*left, *right),
            (Self::String(left), Self::String(right)) => left == right,
            (Self::Symbol(left), Self::Symbol(right)) => left == right,
            (Self::Object(left), Self::Object(right)) => Rc::ptr_eq(left, right),
            (Self::Array(left), Self::Array(right)) => left.ptr_eq(right),
            (Self::Closed(left), Self::Closed(right)) => {
                left.identity_key() == right.identity_key()
            }
            (Self::JsonProjection(left), Self::JsonProjection(right)) => left.ptr_eq(right),
            _ => false,
        }
    }
}

impl JsHash for JsValue {
    fn js_hash(&self) -> u64 {
        match self {
            Self::Undefined => 0x11,
            Self::Null => 0x12,
            Self::Bool(value) => value.js_hash(),
            Self::Number(value) => value.js_hash(),
            Self::String(value) => value.js_hash(),
            Self::Symbol(value) => value.js_hash(),
            Self::Object(value) => hash_identity(Rc::as_ptr(value) as usize),
            Self::Array(value) => value.js_hash(),
            Self::Closed(value) => hash_identity(value.identity_key()),
            Self::JsonProjection(value) => hash_identity(value.identity()),
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
            (Self::Symbol(left), Self::Symbol(right)) => left == right,
            (Self::Object(left), Self::Object(right)) => Rc::ptr_eq(left, right),
            (Self::Array(left), Self::Array(right)) => left.ptr_eq(right),
            (Self::Closed(left), Self::Closed(right)) => {
                left.identity_key() == right.identity_key()
            }
            (Self::JsonProjection(left), Self::JsonProjection(right)) => left.ptr_eq(right),
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

impl From<Null> for JsValue {
    fn from(_: Null) -> Self {
        Self::Null
    }
}

impl From<String> for JsValue {
    fn from(value: String) -> Self {
        Self::String(JsString::from_utf8(&value))
    }
}

impl From<JsString> for JsValue {
    fn from(value: JsString) -> Self {
        Self::String(value)
    }
}

impl From<JsSymbol> for JsValue {
    fn from(value: JsSymbol) -> Self {
        Self::Symbol(value)
    }
}

impl From<Undefined> for JsValue {
    fn from(_: Undefined) -> Self {
        Self::Undefined
    }
}

pub fn from_string(value: &str) -> JsValue {
    JsValue::String(JsString::from_utf8(value))
}

pub fn from_exact_string(value: &JsString) -> JsValue {
    JsValue::String(value.clone())
}

pub fn clone_value(value: &JsValue) -> JsValue {
    value.clone()
}

pub fn from_closed<T>(value: &T) -> JsValue
where
    T: JsClosedValueCarrier + Clone + 'static,
{
    JsValue::closed(value.clone())
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
