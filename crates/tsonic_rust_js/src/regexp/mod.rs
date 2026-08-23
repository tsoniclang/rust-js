use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::ops::Deref;
use std::rc::Rc;
use std::sync::Arc;

use regress::{Flags, Match, Regex};
use tsonic_rust_runtime::Undefined;

use crate::array::JsArray;
use crate::equality::{hash_identity, JsHash, JsSameValueZero, JsStrictEqual};
use crate::errors::{range_error, syntax_error, type_error, JsResult};
use crate::{JsObject, JsString, JsValue};

pub type JsRegExpIndexPair = (f64, f64);

#[derive(Debug, Clone)]
pub struct JsRegExpNamedGroups {
    values: Rc<RefCell<BTreeMap<JsString, Option<JsString>>>>,
}

impl PartialEq for JsRegExpNamedGroups {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.values, &other.values)
    }
}

impl Eq for JsRegExpNamedGroups {}

impl JsSameValueZero for JsRegExpNamedGroups {
    fn same_value_zero(&self, other: &Self) -> bool {
        self == other
    }
}

impl JsHash for JsRegExpNamedGroups {
    fn js_hash(&self) -> u64 {
        hash_identity(Rc::as_ptr(&self.values) as usize)
    }
}

impl JsStrictEqual for JsRegExpNamedGroups {
    fn strict_equal(&self, other: &Self) -> bool {
        self == other
    }
}

impl JsRegExpNamedGroups {
    pub fn get(&self, name: &JsString) -> Option<JsString> {
        self.values.borrow().get(name).cloned().flatten()
    }

    pub fn set(&self, name: &JsString, value: Option<JsString>) {
        self.values.borrow_mut().insert(name.clone(), value);
    }

    pub fn delete(&self, name: &JsString) -> bool {
        self.values.borrow_mut().remove(name);
        true
    }

    pub fn has(&self, name: &JsString) -> bool {
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
pub struct JsRegExpNamedIndices {
    values: Rc<RefCell<BTreeMap<JsString, Option<JsRegExpIndexPair>>>>,
}

impl PartialEq for JsRegExpNamedIndices {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.values, &other.values)
    }
}

impl Eq for JsRegExpNamedIndices {}

impl JsSameValueZero for JsRegExpNamedIndices {
    fn same_value_zero(&self, other: &Self) -> bool {
        self == other
    }
}

impl JsHash for JsRegExpNamedIndices {
    fn js_hash(&self) -> u64 {
        hash_identity(Rc::as_ptr(&self.values) as usize)
    }
}

impl JsStrictEqual for JsRegExpNamedIndices {
    fn strict_equal(&self, other: &Self) -> bool {
        self == other
    }
}

impl JsRegExpNamedIndices {
    pub fn get(&self, name: &JsString) -> Option<JsRegExpIndexPair> {
        self.values.borrow().get(name).copied().flatten()
    }

    pub fn set(&self, name: &JsString, value: Option<JsRegExpIndexPair>) {
        self.values.borrow_mut().insert(name.clone(), value);
    }

    pub fn delete(&self, name: &JsString) -> bool {
        self.values.borrow_mut().remove(name);
        true
    }

    pub fn has(&self, name: &JsString) -> bool {
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
pub struct JsRegExpIndices {
    values: JsArray<JsRegExpIndexPair>,
    groups: Option<JsRegExpNamedIndices>,
}

impl JsRegExpIndices {
    pub fn at(&self, index: usize) -> Option<JsRegExpIndexPair> {
        self.values.get(index)
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn groups(&self) -> Option<JsRegExpNamedIndices> {
        self.groups.clone()
    }
}

impl Deref for JsRegExpIndices {
    type Target = JsArray<JsRegExpIndexPair>;

    fn deref(&self) -> &Self::Target {
        &self.values
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct JsRegExpMatchArray {
    values: JsArray<JsString>,
    index: Option<f64>,
    input: Option<JsString>,
    groups: Option<JsRegExpNamedGroups>,
    indices: Option<JsRegExpIndices>,
    end: usize,
}

impl JsRegExpMatchArray {
    pub fn required_group(&self, index: f64) -> JsString {
        self.values.get_number(index).unwrap_or_default()
    }

    pub fn text(&self) -> JsString {
        self.values.get(0).unwrap_or_default()
    }

    pub fn value(&self) -> JsString {
        self.text()
    }

    pub fn index(&self) -> Option<f64> {
        self.index
    }

    pub fn input(&self) -> Option<JsString> {
        self.input.clone()
    }

    pub fn group(&self, index: usize) -> Option<JsString> {
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

    pub fn groups(&self) -> Option<JsRegExpNamedGroups> {
        self.groups.clone()
    }

    pub fn indices(&self) -> Option<JsRegExpIndices> {
        self.indices.clone()
    }

    pub fn array(&self) -> JsArray<JsString> {
        self.values.clone()
    }
}

impl Deref for JsRegExpMatchArray {
    type Target = JsArray<JsString>;

    fn deref(&self) -> &Self::Target {
        &self.values
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct JsRegExpExecArray {
    values: JsArray<JsString>,
    index: f64,
    input: JsString,
    groups: Option<JsRegExpNamedGroups>,
    indices: Option<JsRegExpIndices>,
    end: usize,
}

impl JsRegExpExecArray {
    pub fn required_group(&self, index: f64) -> JsString {
        self.values.get_number(index).unwrap_or_default()
    }

    pub fn text(&self) -> JsString {
        self.values.get(0).unwrap_or_default()
    }

    pub fn value(&self) -> JsString {
        self.text()
    }

    pub fn index(&self) -> f64 {
        self.index
    }

    pub fn input(&self) -> JsString {
        self.input.clone()
    }

    pub fn group(&self, index: usize) -> Option<JsString> {
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

    pub fn groups(&self) -> Option<JsRegExpNamedGroups> {
        self.groups.clone()
    }

    pub fn indices(&self) -> Option<JsRegExpIndices> {
        self.indices.clone()
    }

    fn into_match_array(self) -> JsRegExpMatchArray {
        JsRegExpMatchArray {
            values: self.values,
            index: Some(self.index),
            input: Some(self.input),
            groups: self.groups,
            indices: self.indices,
            end: self.end,
        }
    }
}

pub fn regexp_exec_into_match_array(value: JsRegExpExecArray) -> JsRegExpMatchArray {
    value.into_match_array()
}

impl Deref for JsRegExpExecArray {
    type Target = JsArray<JsString>;

    fn deref(&self) -> &Self::Target {
        &self.values
    }
}

#[derive(Debug, Clone, Copy)]
struct ParsedFlags {
    has_indices: bool,
    global: bool,
    ignore_case: bool,
    multiline: bool,
    dot_all: bool,
    unicode: bool,
    unicode_sets: bool,
    sticky: bool,
}

impl ParsedFlags {
    fn parse(flags: &JsString) -> JsResult<Self> {
        let mut seen = [false; 128];
        let mut parsed = Self {
            has_indices: false,
            global: false,
            ignore_case: false,
            multiline: false,
            dot_all: false,
            unicode: false,
            unicode_sets: false,
            sticky: false,
        };
        for unit in flags.units() {
            let index = *unit as usize;
            if index >= seen.len() || seen[index] {
                return Err(syntax_error("invalid or duplicate regular-expression flag"));
            }
            seen[index] = true;
            match index as u8 as char {
                'd' => parsed.has_indices = true,
                'g' => parsed.global = true,
                'i' => parsed.ignore_case = true,
                'm' => parsed.multiline = true,
                's' => parsed.dot_all = true,
                'u' => parsed.unicode = true,
                'v' => parsed.unicode_sets = true,
                'y' => parsed.sticky = true,
                _ => return Err(syntax_error("invalid regular-expression flag")),
            }
        }
        if parsed.unicode && parsed.unicode_sets {
            return Err(syntax_error(
                "regular-expression flags 'u' and 'v' are mutually exclusive",
            ));
        }
        Ok(parsed)
    }

    fn canonical(self) -> JsString {
        let mut value = String::with_capacity(8);
        if self.has_indices {
            value.push('d');
        }
        if self.global {
            value.push('g');
        }
        if self.ignore_case {
            value.push('i');
        }
        if self.multiline {
            value.push('m');
        }
        if self.dot_all {
            value.push('s');
        }
        if self.unicode {
            value.push('u');
        }
        if self.unicode_sets {
            value.push('v');
        }
        if self.sticky {
            value.push('y');
        }
        value.into()
    }

    fn engine_flags(self) -> Flags {
        Flags {
            icase: self.ignore_case,
            multiline: self.multiline,
            dot_all: self.dot_all,
            no_opt: false,
            unicode: self.unicode,
            unicode_sets: self.unicode_sets,
        }
    }

    fn full_unicode(self) -> bool {
        self.unicode || self.unicode_sets
    }
}

#[derive(Debug)]
struct CompiledRegExp {
    source: JsString,
    display_source: JsString,
    flags: JsString,
    parsed_flags: ParsedFlags,
    regex: Regex,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct CompiledRegExpKey {
    source: JsString,
    flags: JsString,
}

#[derive(Debug)]
struct CompiledRegExpCacheEntry {
    program: Arc<CompiledRegExp>,
    source_code_units: usize,
}

#[derive(Debug, Default)]
struct CompiledRegExpCache {
    entries: HashMap<CompiledRegExpKey, CompiledRegExpCacheEntry>,
    recency: VecDeque<CompiledRegExpKey>,
    source_code_units: usize,
}

impl CompiledRegExpCache {
    const MAXIMUM_ENTRIES: usize = 256;
    const MAXIMUM_SOURCE_CODE_UNITS: usize = 4_194_304;

    fn get(&mut self, key: &CompiledRegExpKey) -> Option<Arc<CompiledRegExp>> {
        let program = Arc::clone(&self.entries.get(key)?.program);
        self.touch(key);
        Some(program)
    }

    fn insert(&mut self, key: CompiledRegExpKey, program: Arc<CompiledRegExp>) {
        let source_code_units = key.source.len().saturating_add(key.flags.len());
        if source_code_units > Self::MAXIMUM_SOURCE_CODE_UNITS {
            return;
        }
        self.source_code_units = self.source_code_units.saturating_add(source_code_units);
        self.recency.push_back(key.clone());
        self.entries.insert(
            key,
            CompiledRegExpCacheEntry {
                program,
                source_code_units,
            },
        );
        while self.entries.len() > Self::MAXIMUM_ENTRIES
            || self.source_code_units > Self::MAXIMUM_SOURCE_CODE_UNITS
        {
            let Some(oldest) = self.recency.pop_front() else {
                break;
            };
            if let Some(removed) = self.entries.remove(&oldest) {
                self.source_code_units = self
                    .source_code_units
                    .saturating_sub(removed.source_code_units);
            }
        }
    }

    fn touch(&mut self, key: &CompiledRegExpKey) {
        self.recency.retain(|candidate| candidate != key);
        self.recency.push_back(key.clone());
    }
}

thread_local! {
    static COMPILED_REGEXP_CACHE: RefCell<CompiledRegExpCache> =
        RefCell::new(CompiledRegExpCache::default());
}

#[derive(Debug)]
struct JsRegExpState {
    compiled: Arc<CompiledRegExp>,
    last_index: Cell<f64>,
}

#[derive(Debug, Clone)]
pub struct JsRegExp {
    state: Rc<JsRegExpState>,
}

impl PartialEq for JsRegExp {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.state, &other.state)
    }
}

impl Eq for JsRegExp {}

impl JsSameValueZero for JsRegExp {
    fn same_value_zero(&self, other: &Self) -> bool {
        self == other
    }
}

impl JsHash for JsRegExp {
    fn js_hash(&self) -> u64 {
        hash_identity(Rc::as_ptr(&self.state) as usize)
    }
}

impl JsStrictEqual for JsRegExp {
    fn strict_equal(&self, other: &Self) -> bool {
        self == other
    }
}

impl JsRegExp {
    pub fn new<P, F>(pattern: P, flags: F) -> JsResult<Self>
    where
        P: Into<JsString>,
        F: Into<JsString>,
    {
        let pattern = pattern.into();
        if pattern.len() > 1_048_576 {
            return Err(range_error(
                "regular-expression pattern exceeds the deterministic compilation budget",
            ));
        }
        let source_flags = flags.into();
        let parsed_flags = ParsedFlags::parse(&source_flags)?;
        let flags = parsed_flags.canonical();
        let key = CompiledRegExpKey {
            source: pattern.clone(),
            flags: flags.clone(),
        };
        if let Some(compiled) = COMPILED_REGEXP_CACHE.with(|cache| cache.borrow_mut().get(&key)) {
            return Ok(Self::from_compiled(compiled, 0.0));
        }
        let regex = Regex::from_unicode(
            pattern_code_points(&pattern, parsed_flags.full_unicode()).into_iter(),
            parsed_flags.engine_flags(),
        )
        .map_err(|error| syntax_error(format!("invalid ECMAScript regular expression: {error}")))?;
        let compiled = Arc::new(CompiledRegExp {
            display_source: escape_source(&pattern),
            source: pattern,
            flags,
            parsed_flags,
            regex,
        });
        COMPILED_REGEXP_CACHE.with(|cache| {
            cache.borrow_mut().insert(key, Arc::clone(&compiled));
        });
        Ok(Self::from_compiled(compiled, 0.0))
    }

    fn from_compiled(compiled: Arc<CompiledRegExp>, last_index: f64) -> Self {
        Self {
            state: Rc::new(JsRegExpState {
                compiled,
                last_index: Cell::new(last_index),
            }),
        }
    }

    pub fn empty() -> JsResult<Self> {
        Self::new(JsString::new(), JsString::new())
    }

    pub fn from_string(pattern: &JsString) -> JsResult<Self> {
        Self::new(pattern.clone(), JsString::new())
    }

    pub fn from_string_with_flags(pattern: &JsString, flags: &JsString) -> JsResult<Self> {
        Self::new(pattern.clone(), flags.clone())
    }

    pub fn from_string_with_undefined_flags(
        pattern: &JsString,
        _flags: Undefined,
    ) -> JsResult<Self> {
        Self::from_string(pattern)
    }

    pub fn from_undefined(_pattern: Undefined) -> JsResult<Self> {
        Self::empty()
    }

    pub fn from_undefined_with_flags(_pattern: Undefined, flags: &JsString) -> JsResult<Self> {
        Self::new(JsString::new(), flags.clone())
    }

    pub fn from_undefined_with_undefined_flags(
        _pattern: Undefined,
        _flags: Undefined,
    ) -> JsResult<Self> {
        Self::empty()
    }

    pub fn call_from_regexp(pattern: &Self) -> JsResult<Self> {
        Ok(pattern.clone())
    }

    pub fn call_from_regexp_with_flags(pattern: &Self, flags: &JsString) -> JsResult<Self> {
        Self::new(pattern.pattern(), flags.clone())
    }

    pub fn call_from_regexp_with_undefined_flags(
        pattern: &Self,
        _flags: Undefined,
    ) -> JsResult<Self> {
        Self::call_from_regexp(pattern)
    }

    pub fn construct_from_regexp(pattern: &Self) -> JsResult<Self> {
        Self::new(pattern.pattern(), pattern.flags())
    }

    pub fn construct_from_regexp_with_flags(pattern: &Self, flags: &JsString) -> JsResult<Self> {
        Self::new(pattern.pattern(), flags.clone())
    }

    pub fn construct_from_regexp_with_undefined_flags(
        pattern: &Self,
        _flags: Undefined,
    ) -> JsResult<Self> {
        Self::construct_from_regexp(pattern)
    }

    pub fn source(&self) -> JsString {
        self.state.compiled.display_source.clone()
    }
    pub fn pattern(&self) -> JsString {
        self.state.compiled.source.clone()
    }
    pub fn flags(&self) -> JsString {
        self.state.compiled.flags.clone()
    }
    pub fn global(&self) -> bool {
        self.state.compiled.parsed_flags.global
    }
    pub fn has_indices(&self) -> bool {
        self.state.compiled.parsed_flags.has_indices
    }
    pub fn ignore_case(&self) -> bool {
        self.state.compiled.parsed_flags.ignore_case
    }
    pub fn multiline(&self) -> bool {
        self.state.compiled.parsed_flags.multiline
    }
    pub fn dot_all(&self) -> bool {
        self.state.compiled.parsed_flags.dot_all
    }
    pub fn unicode(&self) -> bool {
        self.state.compiled.parsed_flags.unicode
    }
    pub fn unicode_sets(&self) -> bool {
        self.state.compiled.parsed_flags.unicode_sets
    }
    pub fn sticky(&self) -> bool {
        self.state.compiled.parsed_flags.sticky
    }
    pub fn last_index(&self) -> f64 {
        self.state.last_index.get()
    }
    pub fn set_last_index(&self, value: f64) {
        self.state.last_index.set(value);
    }

    pub fn exec(&self, input: &JsString) -> JsResult<Option<JsRegExpExecArray>> {
        let stateful = self.global() || self.sticky();
        let start = if stateful {
            to_length(self.last_index())
        } else {
            0
        };
        if start > input.len() {
            if stateful {
                self.set_last_index(0.0);
            }
            return Ok(None);
        }
        let Some(found) = self.find(input, start, self.sticky())? else {
            if stateful {
                self.set_last_index(0.0);
            }
            return Ok(None);
        };
        let result = self.build_match(input, found);
        if stateful {
            self.set_last_index(result.end as f64);
        }
        Ok(Some(result))
    }

    pub fn test(&self, input: &JsString) -> JsResult<bool> {
        Ok(self.exec(input)?.is_some())
    }

    pub fn match_result(&self, input: &JsString) -> JsResult<Option<JsRegExpMatchArray>> {
        if !self.global() {
            return Ok(self.exec(input)?.map(JsRegExpExecArray::into_match_array));
        }
        self.set_last_index(0.0);
        let mut values = Vec::new();
        while let Some(found) = self.exec(input)? {
            let text = found.text();
            values.push(Some(text.clone()));
            if text.is_empty() {
                self.advance_empty(input);
            }
        }
        Ok(if values.is_empty() {
            None
        } else {
            Some(JsRegExpMatchArray {
                values: array_from_optional(values),
                index: None,
                input: None,
                groups: None,
                indices: None,
                end: 0,
            })
        })
    }

    pub fn match_all(&self, input: &JsString) -> JsResult<JsRegExpStringIterator> {
        Ok(JsRegExpStringIterator::new(self, input.clone()))
    }

    pub fn match_all_for_string(&self, input: &JsString) -> JsResult<JsRegExpStringIterator> {
        if !self.global() {
            return Err(type_error(
                "String.prototype.matchAll requires a global RegExp",
            ));
        }
        self.match_all(input)
    }

    pub fn replace(&self, input: &JsString, replacement: &JsString) -> JsResult<JsString> {
        replace_core(input, self, |matched| {
            Ok(get_substitution(input, matched, replacement))
        })
    }

    pub fn replace_with<F>(&self, input: &JsString, replacer: F) -> JsResult<JsString>
    where
        F: Fn(JsArray<JsValue>) -> JsString,
    {
        replace_core(input, self, |matched| {
            Ok(replacer(regexp_replacement_arguments(matched, input)))
        })
    }

    pub fn try_replace_with<F>(&self, input: &JsString, replacer: F) -> JsResult<JsString>
    where
        F: Fn(JsArray<JsValue>) -> JsResult<JsString>,
    {
        replace_core(input, self, |matched| {
            replacer(regexp_replacement_arguments(matched, input))
        })
    }

    pub fn replace_all_for_string(
        &self,
        input: &JsString,
        replacement: &JsString,
    ) -> JsResult<JsString> {
        if !self.global() {
            return Err(type_error(
                "String.prototype.replaceAll requires a global RegExp",
            ));
        }
        self.replace(input, replacement)
    }

    pub fn replace_all_for_string_with<F>(
        &self,
        input: &JsString,
        replacer: F,
    ) -> JsResult<JsString>
    where
        F: Fn(JsArray<JsValue>) -> JsString,
    {
        if !self.global() {
            return Err(type_error(
                "String.prototype.replaceAll requires a global RegExp",
            ));
        }
        self.replace_with(input, replacer)
    }

    pub fn try_replace_all_for_string_with<F>(
        &self,
        input: &JsString,
        replacer: F,
    ) -> JsResult<JsString>
    where
        F: Fn(JsArray<JsValue>) -> JsResult<JsString>,
    {
        if !self.global() {
            return Err(type_error(
                "String.prototype.replaceAll requires a global RegExp",
            ));
        }
        self.try_replace_with(input, replacer)
    }

    pub fn split(&self, input: &JsString, limit: Option<f64>) -> JsResult<JsArray<JsString>> {
        let maximum = to_uint32(limit.unwrap_or(u32::MAX as f64));
        let mut output = Vec::new();
        if maximum == 0 {
            return Ok(JsArray::new());
        }
        if input.is_empty() {
            if self.find(input, 0, true)?.is_none() {
                output.push(Some(JsString::new()));
            }
            return Ok(array_from_optional(output));
        }
        let mut segment_start = 0;
        let mut cursor = 0;
        while cursor < input.len() {
            let Some(found) = self.find(input, cursor, true)? else {
                cursor = input.advance_index(cursor, self.full_unicode());
                continue;
            };
            let result = self.build_match(input, found);
            if result.end == segment_start {
                cursor = input.advance_index(cursor, self.full_unicode());
                continue;
            }
            output.push(Some(input.slice(segment_start..cursor)));
            if output.len() as u32 >= maximum {
                break;
            }
            for capture_index in 1..result.len() {
                output.push(result.group(capture_index));
                if output.len() as u32 >= maximum {
                    break;
                }
            }
            if output.len() as u32 >= maximum {
                break;
            }
            segment_start = result.end;
            cursor = result.end;
        }
        if (output.len() as u32) < maximum {
            output.push(Some(input.slice(segment_start..input.len())));
        }
        Ok(array_from_optional(output))
    }

    pub fn split_all(&self, input: &JsString) -> JsResult<JsArray<JsString>> {
        self.split(input, None)
    }

    pub fn split_with_limit(&self, input: &JsString, limit: f64) -> JsResult<JsArray<JsString>> {
        self.split(input, Some(limit))
    }

    pub fn search(&self, input: &JsString) -> JsResult<f64> {
        let previous = self.last_index();
        self.set_last_index(0.0);
        let result = self.exec(input);
        self.set_last_index(previous);
        Ok(result?.map(|matched| matched.index()).unwrap_or(-1.0))
    }

    pub fn to_string_value(&self) -> JsString {
        JsString::concat(&[
            JsString::from("/"),
            self.source(),
            JsString::from("/"),
            self.flags(),
        ])
    }

    pub fn escape(value: &JsString) -> JsString {
        let mut output = Vec::with_capacity(value.len());
        let mut index = 0;
        while index < value.len() {
            let unit = value.units()[index];
            if index == 0 && is_ascii_alphanumeric(unit) {
                append_hex_byte(&mut output, unit);
            } else if is_syntax_character(unit) {
                output.push(b'\\' as u16);
                output.push(unit);
            } else if is_other_punctuator(unit) {
                append_hex_byte(&mut output, unit);
            } else {
                match unit {
                    0x000C => output.extend([b'\\' as u16, b'f' as u16]),
                    0x000A => output.extend([b'\\' as u16, b'n' as u16]),
                    0x000D => output.extend([b'\\' as u16, b'r' as u16]),
                    0x0009 => output.extend([b'\\' as u16, b't' as u16]),
                    0x000B => output.extend([b'\\' as u16, b'v' as u16]),
                    0x0020 => output.extend("\\x20".encode_utf16()),
                    _ if (0xD800..=0xDBFF).contains(&unit)
                        && index + 1 < value.len()
                        && (0xDC00..=0xDFFF).contains(&value.units()[index + 1]) =>
                    {
                        output.push(unit);
                        output.push(value.units()[index + 1]);
                        index += 1;
                    }
                    _ if is_pattern_whitespace(unit) || (0xD800..=0xDFFF).contains(&unit) => {
                        append_unicode_escape(&mut output, unit);
                    }
                    _ => output.push(unit),
                }
            }
            index += 1;
        }
        JsString::from_units(output)
    }

    fn full_unicode(&self) -> bool {
        self.state.compiled.parsed_flags.full_unicode()
    }

    fn advance_empty(&self, input: &JsString) {
        self.set_last_index(
            input.advance_index(to_length(self.last_index()), self.full_unicode()) as f64,
        );
    }

    fn find(&self, input: &JsString, start: usize, sticky: bool) -> JsResult<Option<Match>> {
        let maximum_steps = execution_budget(input.len());
        let found = if self.full_unicode() {
            self.state
                .compiled
                .regex
                .try_find_from_utf16(input.units(), start, maximum_steps)
        } else {
            self.state
                .compiled
                .regex
                .try_find_from_ucs2(input.units(), start, maximum_steps)
        }
        .map_err(|error| range_error(error.to_string()))?;
        Ok(found.filter(|matched| !sticky || matched.start() == start))
    }

    fn build_match(&self, input: &JsString, matched: Match) -> JsRegExpExecArray {
        let end = matched.end();
        let mut values = Vec::with_capacity(matched.captures.len() + 1);
        let mut index_values = Vec::with_capacity(matched.captures.len() + 1);
        for range in matched.groups() {
            values.push(range.clone().map(|span| input.slice(span.clone())));
            index_values.push(range.map(|span| (span.start as f64, span.end as f64)));
        }
        let mut groups = BTreeMap::new();
        let mut named_indices = BTreeMap::new();
        for (name, range) in matched.named_groups() {
            let key = JsString::from(name);
            groups.insert(
                key.clone(),
                range.clone().map(|span| input.slice(span.clone())),
            );
            named_indices.insert(key, range.map(|span| (span.start as f64, span.end as f64)));
        }
        JsRegExpExecArray {
            index: matched.start() as f64,
            input: input.clone(),
            groups: if groups.is_empty() {
                None
            } else {
                Some(JsRegExpNamedGroups {
                    values: Rc::new(RefCell::new(groups)),
                })
            },
            indices: if self.has_indices() {
                Some(JsRegExpIndices {
                    values: array_from_optional(index_values),
                    groups: if named_indices.is_empty() {
                        None
                    } else {
                        Some(JsRegExpNamedIndices {
                            values: Rc::new(RefCell::new(named_indices)),
                        })
                    },
                })
            } else {
                None
            },
            values: array_from_optional(values),
            end,
        }
    }
}

#[derive(Debug)]
struct JsRegExpStringIteratorState {
    expression: JsRegExp,
    input: JsString,
    completed: bool,
    global: bool,
}

#[derive(Debug, Clone)]
pub struct JsRegExpStringIterator {
    state: Rc<RefCell<JsRegExpStringIteratorState>>,
}

impl PartialEq for JsRegExpStringIterator {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.state, &other.state)
    }
}

impl Eq for JsRegExpStringIterator {}

impl JsRegExpStringIterator {
    fn new(expression: &JsRegExp, input: JsString) -> Self {
        let clone = JsRegExp::from_compiled(
            Arc::clone(&expression.state.compiled),
            expression.last_index(),
        );
        Self {
            state: Rc::new(RefCell::new(JsRegExpStringIteratorState {
                expression: clone,
                input,
                completed: false,
                global: expression.global(),
            })),
        }
    }

    pub fn iterator(&self) -> Self {
        self.clone()
    }
}

impl Iterator for JsRegExpStringIterator {
    type Item = JsResult<JsRegExpExecArray>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut state = self.state.borrow_mut();
        if state.completed {
            return None;
        }
        match state.expression.exec(&state.input) {
            Ok(Some(result)) => {
                if !state.global {
                    state.completed = true;
                } else if result.text().is_empty() {
                    state.expression.advance_empty(&state.input);
                }
                Some(Ok(result))
            }
            Ok(None) => {
                state.completed = true;
                None
            }
            Err(error) => {
                state.completed = true;
                Some(Err(error))
            }
        }
    }
}

pub fn regexp_match_string(
    input: &JsString,
    pattern: &JsString,
) -> JsResult<Option<JsRegExpMatchArray>> {
    JsRegExp::new(pattern.clone(), JsString::new())?.match_result(input)
}

pub fn regexp_search_string(input: &JsString, pattern: &JsString) -> JsResult<f64> {
    JsRegExp::new(pattern.clone(), JsString::new())?.search(input)
}

pub fn regexp_replacement_argument_string(arguments: &JsArray<JsValue>, index: usize) -> JsString {
    match arguments.get(index) {
        Some(JsValue::String(value)) => value,
        _ => unreachable!("replacement callback string slot violates its closed runtime ABI"),
    }
}

pub fn regexp_replacement_argument_value(arguments: &JsArray<JsValue>, index: usize) -> JsValue {
    arguments.get(index).unwrap_or(JsValue::Undefined)
}

pub fn regexp_replacement_argument_rest(
    arguments: &JsArray<JsValue>,
    index: usize,
) -> JsArray<JsValue> {
    arguments.slice_from(index as f64)
}

pub fn regexp_named_groups_get(groups: &JsRegExpNamedGroups, name: &str) -> Option<JsString> {
    groups.get(&JsString::from(name))
}

pub fn regexp_named_groups_set(groups: &JsRegExpNamedGroups, value: Option<JsString>, name: &str) {
    groups.set(&JsString::from(name), value);
}

pub fn regexp_named_indices_get(
    groups: &JsRegExpNamedIndices,
    name: &str,
) -> Option<JsRegExpIndexPair> {
    groups.get(&JsString::from(name))
}

pub fn regexp_named_indices_set(
    groups: &JsRegExpNamedIndices,
    value: Option<JsRegExpIndexPair>,
    name: &str,
) {
    groups.set(&JsString::from(name), value);
}

fn replace_core<F>(input: &JsString, expression: &JsRegExp, replacer: F) -> JsResult<JsString>
where
    F: Fn(&JsRegExpExecArray) -> JsResult<JsString>,
{
    if expression.global() {
        expression.set_last_index(0.0);
    }
    let mut matches = Vec::new();
    while let Some(found) = expression.exec(input)? {
        let empty = found.text().is_empty();
        matches.push(found);
        if !expression.global() {
            break;
        }
        if empty {
            expression.advance_empty(input);
        }
    }
    if matches.is_empty() {
        return Ok(input.clone());
    }
    let mut parts = Vec::with_capacity(matches.len() * 2 + 1);
    let mut source_position = 0;
    for matched in matches {
        let position = matched.index() as usize;
        if position < source_position {
            continue;
        }
        parts.push(input.slice(source_position..position));
        parts.push(replacer(&matched)?);
        source_position = matched.end;
    }
    parts.push(input.slice(source_position..input.len()));
    Ok(JsString::concat(&parts))
}

pub(crate) fn string_replacement_arguments(
    matched: &JsString,
    offset: usize,
    input: &JsString,
) -> JsArray<JsValue> {
    JsArray::from_dense(vec![
        JsValue::String(matched.clone()),
        JsValue::Number(offset as f64),
        JsValue::String(input.clone()),
    ])
}

fn regexp_replacement_arguments(matched: &JsRegExpExecArray, input: &JsString) -> JsArray<JsValue> {
    let mut values =
        Vec::with_capacity(matched.len() + 2 + usize::from(matched.groups().is_some()));
    values.push(JsValue::String(matched.text()));
    for capture_index in 1..matched.len() {
        values.push(match matched.group(capture_index) {
            Some(capture) => JsValue::String(capture),
            None => JsValue::Undefined,
        });
    }
    values.push(JsValue::Number(matched.index()));
    values.push(JsValue::String(input.clone()));
    if let Some(groups) = matched.groups() {
        let entries = groups
            .values
            .borrow()
            .iter()
            .map(|(name, value)| {
                (
                    name.clone(),
                    value
                        .as_ref()
                        .map_or(JsValue::Undefined, |value| JsValue::String(value.clone())),
                )
            })
            .collect::<Vec<_>>();
        values.push(JsValue::object(JsObject::from_pairs(entries)));
    }
    JsArray::from_dense(values)
}

fn get_substitution(
    input: &JsString,
    matched: &JsRegExpExecArray,
    replacement: &JsString,
) -> JsString {
    let units = replacement.units();
    let mut output = Vec::with_capacity(units.len());
    let position = matched.index() as usize;
    let mut index = 0;
    while index < units.len() {
        if units[index] != b'$' as u16 || index + 1 >= units.len() {
            output.push(units[index]);
            index += 1;
            continue;
        }
        let next = units[index + 1];
        if next == b'$' as u16 {
            output.push(b'$' as u16);
            index += 2;
        } else if next == b'&' as u16 {
            output.extend_from_slice(matched.text().units());
            index += 2;
        } else if next == b'`' as u16 {
            output.extend_from_slice(input.slice(0..position).units());
            index += 2;
        } else if next == b'\'' as u16 {
            output.extend_from_slice(input.slice(matched.end..input.len()).units());
            index += 2;
        } else if next == b'<' as u16 && matched.groups().is_some() {
            let Some(relative) = units[index + 2..]
                .iter()
                .position(|unit| *unit == b'>' as u16)
            else {
                output.push(b'$' as u16);
                index += 1;
                continue;
            };
            let close = index + 2 + relative;
            let name = JsString::from_units(units[index + 2..close].to_vec());
            if let Some(value) = matched.groups().and_then(|groups| groups.get(&name)) {
                output.extend_from_slice(value.units());
            }
            index = close + 1;
        } else if (b'1' as u16..=b'9' as u16).contains(&next) {
            let mut group = (next - b'0' as u16) as usize;
            let mut consumed = 2;
            if index + 2 < units.len() && (b'0' as u16..=b'9' as u16).contains(&units[index + 2]) {
                let candidate = group * 10 + (units[index + 2] - b'0' as u16) as usize;
                if candidate < matched.len() {
                    group = candidate;
                    consumed = 3;
                }
            }
            if group < matched.len() {
                if let Some(value) = matched.group(group) {
                    output.extend_from_slice(value.units());
                }
                index += consumed;
            } else {
                output.push(b'$' as u16);
                index += 1;
            }
        } else {
            output.push(b'$' as u16);
            index += 1;
        }
    }
    JsString::from_units(output)
}

fn array_from_optional<T>(values: Vec<Option<T>>) -> JsArray<T> {
    let length = values.len();
    let present = values
        .into_iter()
        .enumerate()
        .filter_map(|(index, value)| value.map(|value| (index, value)))
        .collect();
    JsArray::from_sparse(length, present)
}

fn pattern_code_points(pattern: &JsString, unicode: bool) -> Vec<u32> {
    let units = pattern.units();
    let mut output = Vec::with_capacity(units.len());
    let mut index = 0;
    while index < units.len() {
        let first = units[index];
        if unicode && (0xD800..=0xDBFF).contains(&first) && index + 1 < units.len() {
            let second = units[index + 1];
            if (0xDC00..=0xDFFF).contains(&second) {
                output.push(0x10000 + (((first as u32 - 0xD800) << 10) | (second as u32 - 0xDC00)));
                index += 2;
                continue;
            }
        }
        output.push(first as u32);
        index += 1;
    }
    output
}

fn escape_source(pattern: &JsString) -> JsString {
    if pattern.is_empty() {
        return JsString::from("(?:)");
    }
    let mut output = Vec::with_capacity(pattern.len());
    for unit in pattern.units() {
        match *unit {
            0x002F => output.extend([b'\\' as u16, b'/' as u16]),
            0x000A => output.extend("\\n".encode_utf16()),
            0x000D => output.extend("\\r".encode_utf16()),
            0x2028 => output.extend("\\u2028".encode_utf16()),
            0x2029 => output.extend("\\u2029".encode_utf16()),
            value => output.push(value),
        }
    }
    JsString::from_units(output)
}

fn to_length(value: f64) -> usize {
    if value.is_nan() || value <= 0.0 {
        0
    } else if !value.is_finite() || value >= usize::MAX as f64 {
        usize::MAX
    } else {
        value.floor() as usize
    }
}

fn execution_budget(input_length: usize) -> u64 {
    const MINIMUM_STEPS: u64 = 1_000_000;
    const MAXIMUM_STEPS: u64 = 50_000_000;
    const STEPS_PER_CODE_UNIT: u64 = 4_096;
    u64::try_from(input_length)
        .unwrap_or(u64::MAX)
        .saturating_mul(STEPS_PER_CODE_UNIT)
        .clamp(MINIMUM_STEPS, MAXIMUM_STEPS)
}

fn to_uint32(value: f64) -> u32 {
    if !value.is_finite() || value == 0.0 {
        return 0;
    }
    value.trunc().rem_euclid(4_294_967_296.0) as u32
}

fn is_ascii_alphanumeric(unit: u16) -> bool {
    matches!(unit, 0x30..=0x39 | 0x41..=0x5A | 0x61..=0x7A)
}

fn is_syntax_character(unit: u16) -> bool {
    matches!(
        unit,
        0x24 | 0x28..=0x2B | 0x2E | 0x2F | 0x3F | 0x5B | 0x5C | 0x5D | 0x5E | 0x7B | 0x7C | 0x7D
    )
}

fn is_other_punctuator(unit: u16) -> bool {
    matches!(
        unit,
        0x21 | 0x22
            | 0x23
            | 0x25
            | 0x26
            | 0x27
            | 0x2C
            | 0x2D
            | 0x3A
            | 0x3B
            | 0x3C
            | 0x3D
            | 0x3E
            | 0x40
            | 0x60
            | 0x7E
    )
}

fn append_hex_byte(output: &mut Vec<u16>, value: u16) {
    output.extend("\\x".encode_utf16());
    output.push(hex_digit((value >> 4) & 0xF));
    output.push(hex_digit(value & 0xF));
}

fn append_unicode_escape(output: &mut Vec<u16>, value: u16) {
    output.extend("\\u".encode_utf16());
    output.push(hex_digit((value >> 12) & 0xF));
    output.push(hex_digit((value >> 8) & 0xF));
    output.push(hex_digit((value >> 4) & 0xF));
    output.push(hex_digit(value & 0xF));
}

fn hex_digit(value: u16) -> u16 {
    if value < 10 {
        b'0' as u16 + value
    } else {
        b'a' as u16 + value - 10
    }
}

fn is_pattern_whitespace(value: u16) -> bool {
    matches!(value, 0x0009..=0x000D | 0x0020 | 0x00A0 | 0x1680 | 0x2000..=0x200A | 0x2028 | 0x2029 | 0x202F | 0x205F | 0x3000 | 0xFEFF)
}
