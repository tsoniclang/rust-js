use std::cell::RefCell;
use std::collections::BTreeMap;
use std::ops::Deref;
use std::rc::Rc;

use tsonic_rust_runtime::{JsError, Undefined};

use super::{
    JsRegExp, JsRegExpExecArray, JsRegExpIndexPair, JsRegExpIndices, JsRegExpMatchArray,
    JsRegExpNamedGroups, JsRegExpNamedIndices, JsRegExpStringIterator,
};
use crate::array::JsArray;
use crate::equality::{hash_identity, JsHash, JsSameValueZero, JsStrictEqual};
use crate::errors::{type_error, JsResult};
use crate::js_string::to_native_string;
use crate::{JsString, JsValue};

pub type RegExpIndexPair = JsRegExpIndexPair;

#[derive(Debug, Clone)]
pub struct RegExpNamedGroups {
    values: Rc<RefCell<BTreeMap<String, Option<String>>>>,
}

impl PartialEq for RegExpNamedGroups {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.values, &other.values)
    }
}

impl Eq for RegExpNamedGroups {}

impl JsSameValueZero for RegExpNamedGroups {
    fn same_value_zero(&self, other: &Self) -> bool {
        self == other
    }
}

impl JsHash for RegExpNamedGroups {
    fn js_hash(&self) -> u64 {
        hash_identity(Rc::as_ptr(&self.values) as usize)
    }
}

impl JsStrictEqual for RegExpNamedGroups {
    fn strict_equal(&self, other: &Self) -> bool {
        self == other
    }
}

impl RegExpNamedGroups {
    pub fn get(&self, name: &str) -> Option<String> {
        self.values.borrow().get(name).cloned().flatten()
    }

    pub fn set(&self, name: &str, value: Option<String>) {
        self.values.borrow_mut().insert(name.to_owned(), value);
    }

    pub fn delete(&self, name: &str) -> bool {
        self.values.borrow_mut().remove(name);
        true
    }

    pub fn has(&self, name: &str) -> bool {
        self.values.borrow().contains_key(name)
    }

    pub fn len(&self) -> usize {
        self.values.borrow().len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.borrow().is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct RegExpNamedIndices {
    values: Rc<RefCell<BTreeMap<String, Option<RegExpIndexPair>>>>,
}

impl PartialEq for RegExpNamedIndices {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.values, &other.values)
    }
}

impl Eq for RegExpNamedIndices {}

impl JsSameValueZero for RegExpNamedIndices {
    fn same_value_zero(&self, other: &Self) -> bool {
        self == other
    }
}

impl JsHash for RegExpNamedIndices {
    fn js_hash(&self) -> u64 {
        hash_identity(Rc::as_ptr(&self.values) as usize)
    }
}

impl JsStrictEqual for RegExpNamedIndices {
    fn strict_equal(&self, other: &Self) -> bool {
        self == other
    }
}

impl RegExpNamedIndices {
    pub fn get(&self, name: &str) -> Option<RegExpIndexPair> {
        self.values.borrow().get(name).copied().flatten()
    }

    pub fn set(&self, name: &str, value: Option<RegExpIndexPair>) {
        self.values.borrow_mut().insert(name.to_owned(), value);
    }

    pub fn delete(&self, name: &str) -> bool {
        self.values.borrow_mut().remove(name);
        true
    }

    pub fn has(&self, name: &str) -> bool {
        self.values.borrow().contains_key(name)
    }

    pub fn len(&self) -> usize {
        self.values.borrow().len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.borrow().is_empty()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RegExpIndices {
    values: JsArray<RegExpIndexPair>,
    groups: Option<RegExpNamedIndices>,
}

impl RegExpIndices {
    pub fn at(&self, index: usize) -> Option<RegExpIndexPair> {
        self.values.get(index)
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn groups(&self) -> Option<RegExpNamedIndices> {
        self.groups.clone()
    }

    pub fn iter_values(&self) -> std::vec::IntoIter<Option<RegExpIndexPair>> {
        self.values.values().into_iter()
    }
}

impl Deref for RegExpIndices {
    type Target = JsArray<RegExpIndexPair>;

    fn deref(&self) -> &Self::Target {
        &self.values
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RegExpMatchArray {
    values: JsArray<String>,
    index: Option<f64>,
    input: Option<String>,
    groups: Option<RegExpNamedGroups>,
    indices: Option<RegExpIndices>,
}

impl RegExpMatchArray {
    pub fn required_group(&self, index: f64) -> String {
        self.values.get_number(index).unwrap_or_default()
    }

    pub fn text(&self) -> String {
        self.values.get(0).unwrap_or_default()
    }

    pub fn value(&self) -> String {
        self.text()
    }

    pub fn index(&self) -> Option<f64> {
        self.index
    }

    pub fn input(&self) -> Option<String> {
        self.input.clone()
    }

    pub fn group(&self, index: usize) -> Option<String> {
        self.values.get(index)
    }

    pub fn group_count(&self) -> usize {
        self.values.len().saturating_sub(1)
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn groups(&self) -> Option<RegExpNamedGroups> {
        self.groups.clone()
    }

    pub fn indices(&self) -> Option<RegExpIndices> {
        self.indices.clone()
    }

    pub fn array(&self) -> JsArray<String> {
        self.values.clone()
    }

    pub fn iter_values(&self) -> std::vec::IntoIter<Option<String>> {
        self.values.values().into_iter()
    }
}

impl Deref for RegExpMatchArray {
    type Target = JsArray<String>;

    fn deref(&self) -> &Self::Target {
        &self.values
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RegExpExecArray {
    values: JsArray<String>,
    index: f64,
    input: String,
    groups: Option<RegExpNamedGroups>,
    indices: Option<RegExpIndices>,
}

impl RegExpExecArray {
    pub fn required_group(&self, index: f64) -> String {
        self.values.get_number(index).unwrap_or_default()
    }

    pub fn text(&self) -> String {
        self.values.get(0).unwrap_or_default()
    }

    pub fn value(&self) -> String {
        self.text()
    }

    pub fn index(&self) -> f64 {
        self.index
    }

    pub fn input(&self) -> String {
        self.input.clone()
    }

    pub fn group(&self, index: usize) -> Option<String> {
        self.values.get(index)
    }

    pub fn group_count(&self) -> usize {
        self.values.len().saturating_sub(1)
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn groups(&self) -> Option<RegExpNamedGroups> {
        self.groups.clone()
    }

    pub fn indices(&self) -> Option<RegExpIndices> {
        self.indices.clone()
    }

    pub fn iter_values(&self) -> std::vec::IntoIter<Option<String>> {
        self.values.values().into_iter()
    }

    fn into_match_array(self) -> RegExpMatchArray {
        RegExpMatchArray {
            values: self.values,
            index: Some(self.index),
            input: Some(self.input),
            groups: self.groups,
            indices: self.indices,
        }
    }
}

impl Deref for RegExpExecArray {
    type Target = JsArray<String>;

    fn deref(&self) -> &Self::Target {
        &self.values
    }
}

pub fn regexp_exec_into_match_array_native(value: RegExpExecArray) -> RegExpMatchArray {
    value.into_match_array()
}

#[derive(Debug, Clone)]
pub struct RegExpStringIterator {
    inner: JsRegExpStringIterator,
}

impl RegExpStringIterator {
    pub fn iterator(&self) -> Self {
        self.clone()
    }
}

impl Iterator for RegExpStringIterator {
    type Item = JsResult<RegExpExecArray>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .next()
            .map(|result| result.and_then(exact_exec_array_to_native))
    }
}

pub fn regexp_new_native(pattern: &str, flags: &str) -> JsResult<JsRegExp> {
    JsRegExp::new(JsString::from_utf8(pattern), JsString::from_utf8(flags))
}

pub fn regexp_empty_native() -> JsResult<JsRegExp> {
    JsRegExp::empty()
}

pub fn regexp_from_string_native(pattern: &str) -> JsResult<JsRegExp> {
    JsRegExp::from_string(&JsString::from_utf8(pattern))
}

pub fn regexp_from_string_with_flags_native(pattern: &str, flags: &str) -> JsResult<JsRegExp> {
    JsRegExp::from_string_with_flags(&JsString::from_utf8(pattern), &JsString::from_utf8(flags))
}

pub fn regexp_from_string_with_undefined_flags_native(
    pattern: &str,
    flags: Undefined,
) -> JsResult<JsRegExp> {
    JsRegExp::from_string_with_undefined_flags(&JsString::from_utf8(pattern), flags)
}

pub fn regexp_from_exact(pattern: &JsString) -> JsResult<JsRegExp> {
    JsRegExp::from_string(pattern)
}

pub fn regexp_from_exact_with_flags(pattern: &JsString, flags: &str) -> JsResult<JsRegExp> {
    JsRegExp::from_string_with_flags(pattern, &JsString::from_utf8(flags))
}

pub fn regexp_from_exact_with_undefined_flags(
    pattern: &JsString,
    flags: Undefined,
) -> JsResult<JsRegExp> {
    JsRegExp::from_string_with_undefined_flags(pattern, flags)
}

pub fn regexp_from_undefined_native(pattern: Undefined) -> JsResult<JsRegExp> {
    JsRegExp::from_undefined(pattern)
}

pub fn regexp_from_undefined_with_flags_native(
    pattern: Undefined,
    flags: &str,
) -> JsResult<JsRegExp> {
    JsRegExp::from_undefined_with_flags(pattern, &JsString::from_utf8(flags))
}

pub fn regexp_from_undefined_with_undefined_flags_native(
    pattern: Undefined,
    flags: Undefined,
) -> JsResult<JsRegExp> {
    JsRegExp::from_undefined_with_undefined_flags(pattern, flags)
}

pub fn regexp_call_from_regexp_native(pattern: &JsRegExp) -> JsResult<JsRegExp> {
    JsRegExp::call_from_regexp(pattern)
}

pub fn regexp_call_from_regexp_with_flags_native(
    pattern: &JsRegExp,
    flags: &str,
) -> JsResult<JsRegExp> {
    JsRegExp::call_from_regexp_with_flags(pattern, &JsString::from_utf8(flags))
}

pub fn regexp_call_from_regexp_with_undefined_flags_native(
    pattern: &JsRegExp,
    flags: Undefined,
) -> JsResult<JsRegExp> {
    JsRegExp::call_from_regexp_with_undefined_flags(pattern, flags)
}

pub fn regexp_construct_from_regexp_native(pattern: &JsRegExp) -> JsResult<JsRegExp> {
    JsRegExp::construct_from_regexp(pattern)
}

pub fn regexp_construct_from_regexp_with_flags_native(
    pattern: &JsRegExp,
    flags: &str,
) -> JsResult<JsRegExp> {
    JsRegExp::construct_from_regexp_with_flags(pattern, &JsString::from_utf8(flags))
}

pub fn regexp_construct_from_regexp_with_undefined_flags_native(
    pattern: &JsRegExp,
    flags: Undefined,
) -> JsResult<JsRegExp> {
    JsRegExp::construct_from_regexp_with_undefined_flags(pattern, flags)
}

pub fn regexp_test_native(expression: &JsRegExp, input: &str) -> JsResult<bool> {
    expression.test(&JsString::from_utf8(input))
}

pub fn regexp_exec_native(expression: &JsRegExp, input: &str) -> JsResult<Option<RegExpExecArray>> {
    expression
        .exec(&JsString::from_utf8(input))?
        .map(exact_exec_array_to_native)
        .transpose()
}

pub fn regexp_match_native(
    expression: &JsRegExp,
    input: &str,
) -> JsResult<Option<RegExpMatchArray>> {
    expression
        .match_result(&JsString::from_utf8(input))?
        .map(exact_match_array_to_native)
        .transpose()
}

pub fn regexp_match_all_native(
    expression: &JsRegExp,
    input: &str,
) -> JsResult<RegExpStringIterator> {
    Ok(RegExpStringIterator {
        inner: expression.match_all(&JsString::from_utf8(input))?,
    })
}

pub fn regexp_match_all_for_string_native(
    expression: &JsRegExp,
    input: &str,
) -> JsResult<RegExpStringIterator> {
    if !expression.global() {
        return Err(type_error(
            "String.prototype.matchAll requires a global RegExp",
        ));
    }
    regexp_match_all_native(expression, input)
}

pub fn regexp_replace_native(
    expression: &JsRegExp,
    input: &str,
    replacement: &str,
) -> JsResult<String> {
    exact_to_native(
        &expression.replace(
            &JsString::from_utf8(input),
            &JsString::from_utf8(replacement),
        )?,
        "regular-expression replacement result",
    )
}

pub fn regexp_replace_all_for_string_native(
    expression: &JsRegExp,
    input: &str,
    replacement: &str,
) -> JsResult<String> {
    exact_to_native(
        &expression.replace_all_for_string(
            &JsString::from_utf8(input),
            &JsString::from_utf8(replacement),
        )?,
        "regular-expression replacement result",
    )
}

pub fn regexp_try_replace_native_with<E, F>(
    expression: &JsRegExp,
    input: &str,
    replacer: F,
) -> Result<String, E>
where
    E: From<JsError>,
    F: Fn(JsArray<JsValue>) -> Result<String, E>,
{
    let exact_input = JsString::from_utf8(input);
    let result = expression.try_replace_with(&exact_input, |arguments| {
        replacer(arguments).map(|value| JsString::from_utf8(&value))
    })?;
    exact_to_native(&result, "regular-expression replacement result").map_err(E::from)
}

pub fn regexp_try_replace_all_for_string_native_with<E, F>(
    expression: &JsRegExp,
    input: &str,
    replacer: F,
) -> Result<String, E>
where
    E: From<JsError>,
    F: Fn(JsArray<JsValue>) -> Result<String, E>,
{
    let exact_input = JsString::from_utf8(input);
    let result = expression.try_replace_all_for_string_with(&exact_input, |arguments| {
        replacer(arguments).map(|value| JsString::from_utf8(&value))
    })?;
    exact_to_native(&result, "regular-expression replacement result").map_err(E::from)
}

pub fn regexp_split_native(
    expression: &JsRegExp,
    input: &str,
    limit: Option<f64>,
) -> JsResult<JsArray<String>> {
    exact_array_to_native(
        expression.split(&JsString::from_utf8(input), limit)?,
        "regular-expression split result",
    )
}

pub fn regexp_split_all_native(expression: &JsRegExp, input: &str) -> JsResult<JsArray<String>> {
    regexp_split_native(expression, input, None)
}

pub fn regexp_split_with_limit_native(
    expression: &JsRegExp,
    input: &str,
    limit: f64,
) -> JsResult<JsArray<String>> {
    regexp_split_native(expression, input, Some(limit))
}

pub fn regexp_search_native(expression: &JsRegExp, input: &str) -> JsResult<f64> {
    expression.search(&JsString::from_utf8(input))
}

pub fn regexp_match_string_native(
    input: &str,
    pattern: &str,
) -> JsResult<Option<RegExpMatchArray>> {
    regexp_match_native(&regexp_from_string_native(pattern)?, input)
}

pub fn regexp_search_string_native(input: &str, pattern: &str) -> JsResult<f64> {
    regexp_search_native(&regexp_from_string_native(pattern)?, input)
}

pub fn string_match_regexp_native(
    input: &str,
    expression: &JsRegExp,
) -> JsResult<Option<RegExpMatchArray>> {
    regexp_match_native(expression, input)
}

pub fn string_match_all_regexp_native(
    input: &str,
    expression: &JsRegExp,
) -> JsResult<RegExpStringIterator> {
    regexp_match_all_for_string_native(expression, input)
}

pub fn string_replace_regexp_native(
    input: &str,
    expression: &JsRegExp,
    replacement: &str,
) -> JsResult<String> {
    regexp_replace_native(expression, input, replacement)
}

pub fn string_replace_all_regexp_native(
    input: &str,
    expression: &JsRegExp,
    replacement: &str,
) -> JsResult<String> {
    regexp_replace_all_for_string_native(expression, input, replacement)
}

pub fn string_try_replace_regexp_native_with<E, F>(
    input: &str,
    expression: &JsRegExp,
    replacer: F,
) -> Result<String, E>
where
    E: From<JsError>,
    F: Fn(JsArray<JsValue>) -> Result<String, E>,
{
    regexp_try_replace_native_with(expression, input, replacer)
}

pub fn string_try_replace_all_regexp_native_with<E, F>(
    input: &str,
    expression: &JsRegExp,
    replacer: F,
) -> Result<String, E>
where
    E: From<JsError>,
    F: Fn(JsArray<JsValue>) -> Result<String, E>,
{
    regexp_try_replace_all_for_string_native_with(expression, input, replacer)
}

pub fn string_search_regexp_native(input: &str, expression: &JsRegExp) -> JsResult<f64> {
    regexp_search_native(expression, input)
}

pub fn string_split_regexp_native(input: &str, expression: &JsRegExp) -> JsResult<JsArray<String>> {
    regexp_split_all_native(expression, input)
}

pub fn string_split_regexp_with_limit_native(
    input: &str,
    expression: &JsRegExp,
    limit: f64,
) -> JsResult<JsArray<String>> {
    regexp_split_with_limit_native(expression, input, limit)
}

pub fn regexp_source_native(expression: &JsRegExp) -> JsResult<String> {
    exact_to_native(&expression.source(), "RegExp.source")
}

pub fn regexp_flags_native(expression: &JsRegExp) -> String {
    expression
        .flags()
        .to_utf8()
        .expect("canonical regular-expression flags are ASCII")
}

pub fn regexp_to_string_native(expression: &JsRegExp) -> JsResult<String> {
    exact_to_native(
        &expression.to_string_value(),
        "RegExp.prototype.toString result",
    )
}

pub fn regexp_escape_native(value: &str) -> String {
    JsRegExp::escape(&JsString::from_utf8(value))
        .to_utf8()
        .expect("RegExp.escape over a native Rust string remains well-formed")
}

pub fn regexp_escape_exact_native(value: &JsString) -> String {
    JsRegExp::escape(value)
        .to_utf8()
        .expect("RegExp.escape always produces well-formed UTF-16")
}

pub fn regexp_replacement_argument_string_native(
    arguments: &JsArray<JsValue>,
    index: usize,
) -> JsResult<String> {
    match arguments.get(index) {
        Some(JsValue::String(value)) => {
            exact_to_native(&value, "regular-expression replacement callback argument")
        }
        _ => unreachable!("replacement callback string slot violates its closed runtime ABI"),
    }
}

pub fn regexp_named_groups_get_native(groups: &RegExpNamedGroups, name: &str) -> Option<String> {
    groups.get(name)
}

pub fn regexp_named_groups_set_native(
    groups: &RegExpNamedGroups,
    value: Option<String>,
    name: &str,
) {
    groups.set(name, value);
}

pub fn regexp_named_groups_delete_native(groups: &RegExpNamedGroups, name: &str) -> bool {
    groups.delete(name)
}

pub fn regexp_named_indices_get_native(
    groups: &RegExpNamedIndices,
    name: &str,
) -> Option<RegExpIndexPair> {
    groups.get(name)
}

pub fn regexp_named_indices_set_native(
    groups: &RegExpNamedIndices,
    value: Option<RegExpIndexPair>,
    name: &str,
) {
    groups.set(name, value);
}

pub fn regexp_named_indices_delete_native(groups: &RegExpNamedIndices, name: &str) -> bool {
    groups.delete(name)
}

fn exact_exec_array_to_native(value: JsRegExpExecArray) -> JsResult<RegExpExecArray> {
    Ok(RegExpExecArray {
        values: exact_array_to_native(value.values, "regular-expression capture")?,
        index: value.index,
        input: exact_to_native(&value.input, "RegExpExecArray.input")?,
        groups: exact_named_groups_to_native(value.groups)?,
        indices: exact_indices_to_native(value.indices)?,
    })
}

fn exact_match_array_to_native(value: JsRegExpMatchArray) -> JsResult<RegExpMatchArray> {
    Ok(RegExpMatchArray {
        values: exact_array_to_native(value.values, "regular-expression capture")?,
        index: value.index,
        input: value
            .input
            .map(|input| exact_to_native(&input, "RegExpMatchArray.input"))
            .transpose()?,
        groups: exact_named_groups_to_native(value.groups)?,
        indices: exact_indices_to_native(value.indices)?,
    })
}

fn exact_indices_to_native(value: Option<JsRegExpIndices>) -> JsResult<Option<RegExpIndices>> {
    value
        .map(|value| {
            Ok(RegExpIndices {
                values: value.values,
                groups: exact_named_indices_to_native(value.groups)?,
            })
        })
        .transpose()
}

fn exact_named_groups_to_native(
    value: Option<JsRegExpNamedGroups>,
) -> JsResult<Option<RegExpNamedGroups>> {
    value
        .map(|value| {
            let mut converted = BTreeMap::new();
            for (name, group) in value.values.borrow().iter() {
                converted.insert(
                    exact_to_native(name, "regular-expression group name")?,
                    group
                        .as_ref()
                        .map(|group| exact_to_native(group, "named regular-expression capture"))
                        .transpose()?,
                );
            }
            Ok(RegExpNamedGroups {
                values: Rc::new(RefCell::new(converted)),
            })
        })
        .transpose()
}

fn exact_named_indices_to_native(
    value: Option<JsRegExpNamedIndices>,
) -> JsResult<Option<RegExpNamedIndices>> {
    value
        .map(|value| {
            let mut converted = BTreeMap::new();
            for (name, indices) in value.values.borrow().iter() {
                converted.insert(
                    exact_to_native(name, "regular-expression group name")?,
                    *indices,
                );
            }
            Ok(RegExpNamedIndices {
                values: Rc::new(RefCell::new(converted)),
            })
        })
        .transpose()
}

fn exact_array_to_native(
    values: JsArray<JsString>,
    context: &'static str,
) -> JsResult<JsArray<String>> {
    let length = values.len();
    let mut present = Vec::new();
    for (index, value) in values.values().into_iter().enumerate() {
        if let Some(value) = value {
            present.push((index, exact_to_native(&value, context)?));
        }
    }
    Ok(JsArray::from_sparse(length, present))
}

fn exact_to_native(value: &JsString, context: &'static str) -> JsResult<String> {
    to_native_string(value, context)
}
