use std::cell::RefCell;
use std::collections::BTreeMap;
use std::ops::Deref;
use std::rc::Rc;

use tsonic_rust_runtime::{JsError, Undefined};

use super::{JsRegExp, JsRegExpIndexPair};

mod operations;
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
    values: JsArray<Option<RegExpIndexPair>>,
    groups: Option<RegExpNamedIndices>,
}

impl RegExpIndices {
    pub fn at(&self, index: usize) -> Option<RegExpIndexPair> {
        self.values.get(index).flatten()
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

    pub fn iter_values(&self) -> impl Iterator<Item = Option<RegExpIndexPair>> {
        self.values.values().into_iter()
    }
}

impl Deref for RegExpIndices {
    type Target = JsArray<Option<RegExpIndexPair>>;

    fn deref(&self) -> &Self::Target {
        &self.values
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RegExpMatchArray {
    values: JsArray<Option<String>>,
    index: Option<f64>,
    input: Option<String>,
    groups: Option<RegExpNamedGroups>,
    indices: Option<RegExpIndices>,
}

impl RegExpMatchArray {
    pub fn required_group(&self, index: f64) -> String {
        self.values
            .get_number(index)
            .flatten()
            .expect("required whole-match capture")
    }

    pub fn text(&self) -> String {
        self.values
            .get(0)
            .flatten()
            .expect("required whole-match capture")
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
        self.values.get(index).flatten()
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

    pub fn array(&self) -> JsArray<Option<String>> {
        self.values.clone()
    }

    pub fn iter_values(&self) -> impl Iterator<Item = Option<String>> {
        self.values.values().into_iter()
    }
}

impl Deref for RegExpMatchArray {
    type Target = JsArray<Option<String>>;

    fn deref(&self) -> &Self::Target {
        &self.values
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RegExpExecArray {
    values: JsArray<Option<String>>,
    index: f64,
    input: String,
    groups: Option<RegExpNamedGroups>,
    indices: Option<RegExpIndices>,
}

impl RegExpExecArray {
    pub fn required_group(&self, index: f64) -> String {
        self.values
            .get_number(index)
            .flatten()
            .expect("required whole-match capture")
    }

    pub fn text(&self) -> String {
        self.values
            .get(0)
            .flatten()
            .expect("required whole-match capture")
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
        self.values.get(index).flatten()
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

    pub fn iter_values(&self) -> impl Iterator<Item = Option<String>> {
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
    type Target = JsArray<Option<String>>;

    fn deref(&self) -> &Self::Target {
        &self.values
    }
}

pub fn regexp_exec_into_match_array_native(value: RegExpExecArray) -> RegExpMatchArray {
    value.into_match_array()
}

#[derive(Debug, Clone)]
pub struct RegExpStringIterator {
    state: Rc<RefCell<NativeIteratorState>>,
}

#[derive(Debug)]
struct NativeIteratorState {
    expression: JsRegExp,
    input: String,
    done: bool,
}

impl RegExpStringIterator {
    pub fn iterator(&self) -> Self {
        self.clone()
    }
}

impl Iterator for RegExpStringIterator {
    type Item = JsResult<RegExpExecArray>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut state = self.state.borrow_mut();
        if state.done {
            return None;
        }
        match operations::execute(&state.expression, &state.input) {
            Err(error) => {
                state.done = true;
                Some(Err(error))
            }
            Ok(None) => {
                state.done = true;
                None
            }
            Ok(Some(found)) => {
                if !state.expression.global() {
                    state.done = true;
                } else if found.start() == found.end() {
                    state
                        .expression
                        .set_last_index(operations::advance(&state.input, found.end()) as f64);
                }
                Some(Ok(operations::build(
                    &state.expression,
                    &state.input,
                    found,
                )))
            }
        }
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
    Ok(operations::execute(expression, input)?.is_some())
}

pub fn regexp_exec_native(expression: &JsRegExp, input: &str) -> JsResult<Option<RegExpExecArray>> {
    Ok(operations::execute(expression, input)?
        .map(|found| operations::build(expression, input, found)))
}

pub fn regexp_match_native(
    expression: &JsRegExp,
    input: &str,
) -> JsResult<Option<RegExpMatchArray>> {
    if !expression.global() {
        return Ok(regexp_exec_native(expression, input)?.map(RegExpExecArray::into_match_array));
    }
    let matches = operations::collect(expression, input)?;
    if matches.is_empty() {
        return Ok(None);
    }
    Ok(Some(RegExpMatchArray {
        values: JsArray::from_dense(
            matches
                .into_iter()
                .map(|found| Some(input[found.range].to_owned()))
                .collect(),
        ),
        index: None,
        input: None,
        groups: None,
        indices: None,
    }))
}

pub fn regexp_match_all_native(
    expression: &JsRegExp,
    input: &str,
) -> JsResult<RegExpStringIterator> {
    let cloned = JsRegExp::construct_from_regexp(expression)?;
    cloned.set_last_index(expression.last_index());
    Ok(RegExpStringIterator {
        state: Rc::new(RefCell::new(NativeIteratorState {
            expression: cloned,
            input: input.to_owned(),
            done: false,
        })),
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
    let mut output = String::with_capacity(input.len());
    let mut copied = 0;
    if expression.global() {
        expression.set_last_index(0.0);
    }
    while let Some(found) = operations::execute(expression, input)? {
        output.push_str(&input[copied..found.start()]);
        operations::append_substitution(&mut output, input, &found, replacement);
        copied = found.end();
        if !expression.global() {
            break;
        }
        if found.start() == found.end() {
            expression.set_last_index(operations::advance(input, found.end()) as f64);
        }
    }
    output.push_str(&input[copied..]);
    Ok(output)
}

pub fn regexp_replace_all_for_string_native(
    expression: &JsRegExp,
    input: &str,
    replacement: &str,
) -> JsResult<String> {
    if !expression.global() {
        return Err(type_error(
            "String.prototype.replaceAll requires a global RegExp",
        ));
    }
    regexp_replace_native(expression, input, replacement)
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
    let matches = operations::collect(expression, input)?;
    let mut output = String::with_capacity(input.len());
    let mut copied = 0;
    for found in matches {
        output.push_str(&input[copied..found.start()]);
        output.push_str(&replacer(operations::replacement_arguments(input, &found))?);
        copied = found.end();
    }
    output.push_str(&input[copied..]);
    Ok(output)
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
    if !expression.global() {
        return Err(type_error("String.prototype.replaceAll requires a global RegExp").into());
    }
    regexp_try_replace_native_with(expression, input, replacer)
}

pub fn regexp_split_native(
    expression: &JsRegExp,
    input: &str,
    limit: Option<f64>,
) -> JsResult<JsArray<Option<String>>> {
    let maximum = limit.map(crate::native_integer::native_length).transpose()?.unwrap_or(usize::MAX);
    let mut output = Vec::new();
    if maximum == 0 {
        return Ok(JsArray::new());
    }
    if input.is_empty() {
        if operations::find(expression, input, 0, true)?.is_none() {
            output.push(Some(String::new()));
        }
        return Ok(super::array_from_optional(output));
    }
    let mut segment = 0;
    let mut cursor = 0;
    while cursor < input.len() {
        let Some(found) = operations::find(expression, input, cursor, true)? else {
            cursor = operations::advance(input, cursor);
            continue;
        };
        if found.end() == segment {
            cursor = operations::advance(input, cursor);
            continue;
        }
        output.push(Some(input[segment..cursor].to_owned()));
        if output.len() >= maximum {
            break;
        }
        for capture in &found.captures {
            output.push(capture.clone().map(|span| input[span].to_owned()));
            if output.len() >= maximum {
                break;
            }
        }
        if output.len() >= maximum {
            break;
        }
        segment = found.end();
        cursor = found.end();
    }
    if output.len() < maximum {
        output.push(Some(input[segment..].to_owned()));
    }
    Ok(super::array_from_optional(output))
}

pub fn regexp_split_all_native(
    expression: &JsRegExp,
    input: &str,
) -> JsResult<JsArray<Option<String>>> {
    regexp_split_native(expression, input, None)
}

pub fn regexp_split_with_limit_native(
    expression: &JsRegExp,
    input: &str,
    limit: f64,
) -> JsResult<JsArray<Option<String>>> {
    regexp_split_native(expression, input, Some(limit))
}

pub fn regexp_search_native(expression: &JsRegExp, input: &str) -> JsResult<f64> {
    Ok(operations::find(expression, input, 0, expression.sticky())?
        .map_or(-1.0, |found| found.start() as f64))
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

pub fn string_split_regexp_native(
    input: &str,
    expression: &JsRegExp,
) -> JsResult<JsArray<Option<String>>> {
    regexp_split_all_native(expression, input)
}

pub fn string_split_regexp_with_limit_native(
    input: &str,
    expression: &JsRegExp,
    limit: f64,
) -> JsResult<JsArray<Option<String>>> {
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
        Some(JsValue::String(value)) => Ok(value),
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

fn exact_to_native(value: &JsString, context: &'static str) -> JsResult<String> {
    to_native_string(value, context)
}
