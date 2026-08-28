//! Closed JSON parser/stringifier for supported carrier values.

use std::collections::HashSet;
use std::rc::Rc;

use crate::errors::{range_error, syntax_error, type_error, JsResult};
use crate::object::JsObject;
use crate::value::JsValue;
use crate::JsString;
use tsonic_rust_runtime::{TsonicError, TsonicResult};

pub const JSON_MAX_INPUT_BYTES: usize = 16 * 1024 * 1024;
pub const JSON_MAX_OUTPUT_BYTES: usize = 16 * 1024 * 1024;
pub const JSON_MAX_DEPTH: usize = 256;
pub const JSON_MAX_NODES: usize = 1_000_000;
pub const JSON_MAX_MEMBERS: usize = 1_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JsonLimits {
    pub max_input_bytes: usize,
    pub max_output_bytes: usize,
    pub max_depth: usize,
    pub max_nodes: usize,
    pub max_members: usize,
}

impl Default for JsonLimits {
    fn default() -> Self {
        Self {
            max_input_bytes: JSON_MAX_INPUT_BYTES,
            max_output_bytes: JSON_MAX_OUTPUT_BYTES,
            max_depth: JSON_MAX_DEPTH,
            max_nodes: JSON_MAX_NODES,
            max_members: JSON_MAX_MEMBERS,
        }
    }
}

pub fn parse(text: &str) -> JsResult<JsValue> {
    parse_with_limits(text, JsonLimits::default())
}

pub fn parse_with_limits(text: &str, limits: JsonLimits) -> JsResult<JsValue> {
    if text.len() > limits.max_input_bytes {
        return Err(range_error("JSON input exceeds the configured byte limit"));
    }
    let exact = JsString::from_utf8(text);
    let mut parser = Parser::new(&exact, limits);
    let value = parser.parse_value(0)?;
    parser.skip_ws();
    if parser.is_done() {
        Ok(value)
    } else {
        Err(syntax_error("JSON.parse found trailing input"))
    }
}

pub fn stringify(value: &JsValue) -> JsResult<Option<String>> {
    stringify_with_options::<tsonic_rust_runtime::JsError>(
        value,
        "",
        JsonLimits::default(),
        None,
        None,
    )
}

pub fn stringify_pretty(value: &JsValue) -> JsResult<Option<String>> {
    stringify(value)
}

pub fn stringify_with_indent(value: &JsValue, indent: &str) -> JsResult<Option<String>> {
    stringify_with_indent_and_limits(value, indent, JsonLimits::default())
}

pub fn stringify_with_limits(value: &JsValue, limits: JsonLimits) -> JsResult<Option<String>> {
    stringify_with_indent_and_limits(value, "", limits)
}

pub fn stringify_with_indent_and_limits(
    value: &JsValue,
    indent: &str,
    limits: JsonLimits,
) -> JsResult<Option<String>> {
    stringify_with_options::<tsonic_rust_runtime::JsError>(
        value,
        indent,
        limits,
        None,
        None,
    )
}

pub fn stringify_with_space_number(value: &JsValue, space: f64) -> JsResult<Option<String>> {
    stringify_with_indent(value, &number_indent(space))
}

pub fn stringify_with_space_string(value: &JsValue, space: &str) -> JsResult<Option<String>> {
    let indent = string_indent(space)?;
    stringify_with_indent(value, &indent)
}

pub fn stringify_with_property_list(
    value: &JsValue,
    property_list: &JsValue,
) -> JsResult<Option<String>> {
    let properties = normalize_property_list(property_list)?;
    stringify_with_options::<tsonic_rust_runtime::JsError>(
        value,
        "",
        JsonLimits::default(),
        None,
        Some(&properties),
    )
}

pub fn stringify_with_property_list_and_space_number(
    value: &JsValue,
    property_list: &JsValue,
    space: f64,
) -> JsResult<Option<String>> {
    let properties = normalize_property_list(property_list)?;
    stringify_with_options::<tsonic_rust_runtime::JsError>(
        value,
        &number_indent(space),
        JsonLimits::default(),
        None,
        Some(&properties),
    )
}

pub fn stringify_with_property_list_and_space_string(
    value: &JsValue,
    property_list: &JsValue,
    space: &str,
) -> JsResult<Option<String>> {
    let properties = normalize_property_list(property_list)?;
    let indent = string_indent(space)?;
    stringify_with_options::<tsonic_rust_runtime::JsError>(
        value,
        &indent,
        JsonLimits::default(),
        None,
        Some(&properties),
    )
}

pub fn stringify_with_replacer(
    value: &JsValue,
    mut replacer: impl FnMut(String, JsValue) -> JsValue,
) -> JsResult<Option<String>> {
    let mut replacer = |key: String, value: JsValue| Ok(replacer(key, value));
    stringify_with_options(
        value,
        "",
        JsonLimits::default(),
        Some(&mut replacer),
        None,
    )
}

pub fn stringify_with_replacer_and_space_number(
    value: &JsValue,
    mut replacer: impl FnMut(String, JsValue) -> JsValue,
    space: f64,
) -> JsResult<Option<String>> {
    let mut replacer = |key: String, value: JsValue| Ok(replacer(key, value));
    stringify_with_options(
        value,
        &number_indent(space),
        JsonLimits::default(),
        Some(&mut replacer),
        None,
    )
}

pub fn stringify_with_replacer_and_space_string(
    value: &JsValue,
    mut replacer: impl FnMut(String, JsValue) -> JsValue,
    space: &str,
) -> JsResult<Option<String>> {
    let indent = string_indent(space)?;
    let mut replacer = |key: String, value: JsValue| Ok(replacer(key, value));
    stringify_with_options(
        value,
        &indent,
        JsonLimits::default(),
        Some(&mut replacer),
        None,
    )
}

pub fn try_stringify_with_replacer(
    value: &JsValue,
    mut replacer: impl FnMut(String, JsValue) -> TsonicResult<JsValue>,
) -> TsonicResult<Option<String>> {
    stringify_with_options(
        value,
        "",
        JsonLimits::default(),
        Some(&mut replacer),
        None,
    )
}

pub fn try_stringify_with_replacer_and_space_number(
    value: &JsValue,
    mut replacer: impl FnMut(String, JsValue) -> TsonicResult<JsValue>,
    space: f64,
) -> TsonicResult<Option<String>> {
    stringify_with_options(
        value,
        &number_indent(space),
        JsonLimits::default(),
        Some(&mut replacer),
        None,
    )
}

pub fn try_stringify_with_replacer_and_space_string(
    value: &JsValue,
    mut replacer: impl FnMut(String, JsValue) -> TsonicResult<JsValue>,
    space: &str,
) -> TsonicResult<Option<String>> {
    let indent = string_indent(space).map_err(TsonicError::from)?;
    stringify_with_options(
        value,
        &indent,
        JsonLimits::default(),
        Some(&mut replacer),
        None,
    )
}

fn stringify_with_options<E>(
    value: &JsValue,
    indent: &str,
    limits: JsonLimits,
    replacer: Option<&mut dyn FnMut(String, JsValue) -> Result<JsValue, E>>,
    property_list: Option<&[JsString]>,
) -> Result<Option<String>, E>
where
    E: From<tsonic_rust_runtime::JsError>,
{
    let indent = JsString::from_utf8(indent);
    if indent.len() > 10 {
        return Err(type_error(
            "JSON indentation must be pre-resolved to at most 10 UTF-16 code units",
        ).into());
    }
    let mut serializer = Serializer::new(indent, limits, replacer, property_list);
    if serializer.serialize_property(&JsString::from_utf8(""), value, 0)? {
        let output = String::from_utf16(&serializer.output)
            .map_err(|_| E::from(type_error("JSON serialization produced an invalid native Rust string")))?;
        Ok(Some(output))
    } else {
        Ok(None)
    }
}

fn number_indent(space: f64) -> String {
    " ".repeat(space.trunc().clamp(0.0, 10.0) as usize)
}

fn string_indent(space: &str) -> JsResult<String> {
    let units = space.encode_utf16().take(10).collect::<Vec<_>>();
    String::from_utf16(&units).map_err(|_| {
        type_error("JSON indentation truncated through a Unicode scalar boundary")
    })
}

fn normalize_property_list(
    values: &JsValue,
) -> JsResult<Vec<JsString>> {
    let values = values.as_array().ok_or_else(|| {
        type_error("JSON property-list replacer requires an exact JavaScript array value")
    })?;
    let mut properties = Vec::new();
    for value in values.values().into_iter().flatten() {
        let key = match value {
            JsValue::String(value) => value,
            JsValue::Number(value) => JsString::from_utf8(&crate::number::to_string(value)),
            _ => continue,
        };
        if !properties.contains(&key) {
            properties.push(key);
        }
    }
    Ok(properties)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum ContainerId {
    Object(usize),
    Array(usize),
    Projection(usize),
}

struct Serializer<'a, E> {
    indent: JsString,
    limits: JsonLimits,
    output: Vec<u16>,
    output_bytes: usize,
    active: HashSet<ContainerId>,
    nodes: usize,
    members: usize,
    replacer: Option<&'a mut dyn FnMut(String, JsValue) -> Result<JsValue, E>>,
    property_list: Option<&'a [JsString]>,
}

impl<'a, E> Serializer<'a, E>
where
    E: From<tsonic_rust_runtime::JsError>,
{
    fn new(
        indent: JsString,
        limits: JsonLimits,
        replacer: Option<&'a mut dyn FnMut(String, JsValue) -> Result<JsValue, E>>,
        property_list: Option<&'a [JsString]>,
    ) -> Self {
        Self {
            indent,
            limits,
            output: Vec::new(),
            output_bytes: 0,
            active: HashSet::new(),
            nodes: 0,
            members: 0,
            replacer,
            property_list,
        }
    }

    fn serialize_property(
        &mut self,
        key: &JsString,
        value: &JsValue,
        depth: usize,
    ) -> Result<bool, E> {
        let projection = match value {
            JsValue::JsonProjection(projection) => {
                Some(ContainerId::Projection(projection.identity()))
            }
            _ => None,
        };
        if let Some(id) = projection {
            if !self.active.insert(id) {
                return Err(type_error("Converting circular structure to JSON").into());
            }
        }
        let result = self
            .replaced_value(key, value)
            .and_then(|value| self.serialize_value(&value, depth));
        if let Some(id) = projection {
            self.active.remove(&id);
        }
        result
    }

    fn replaced_value(
        &mut self,
        key: &JsString,
        value: &JsValue,
    ) -> Result<JsValue, E> {
        let value = match value {
            JsValue::JsonProjection(projection) => {
                let key = key.to_utf8().map_err(|_| {
                    E::from(type_error(
                        "JSON projection key cannot be represented by the native Rust string carrier",
                    ))
                })?;
                projection.project(key).map_err(E::from)?
            }
            _ => value.clone(),
        };
        Ok(match self.replacer.as_mut() {
            Some(replacer) => {
                let key = key.to_utf8().map_err(|_| {
                    E::from(type_error(
                        "JSON replacer key cannot be represented by the native Rust string carrier",
                    ))
                })?;
                replacer(key, value)?
            }
            None => value,
        })
    }

    fn serialize_value(&mut self, value: &JsValue, depth: usize) -> Result<bool, E> {
        self.count_node(depth)?;
        match value {
            JsValue::Undefined | JsValue::Symbol(_) => Ok(false),
            JsValue::Null => {
                self.push_str("null")?;
                Ok(true)
            }
            JsValue::Bool(value) => {
                self.push_str(if *value { "true" } else { "false" })?;
                Ok(true)
            }
            JsValue::Number(value) => {
                self.push_str(&json_number(*value))?;
                Ok(true)
            }
            JsValue::String(value) => {
                self.push_quoted(value)?;
                Ok(true)
            }
            JsValue::Array(values) => {
                let id = ContainerId::Array(values.identity());
                self.with_container(id, |serializer| {
                    serializer.push_char('[')?;
                    let mut first = true;
                    for (index, value) in values.values().into_iter().enumerate() {
                        serializer.count_member()?;
                        serializer.member_prefix(depth, &mut first)?;
                        let value = value.unwrap_or(JsValue::Undefined);
                        if !serializer.serialize_property(
                            &JsString::from_utf8(&index.to_string()),
                            &value,
                            depth + 1,
                        )? {
                            serializer.push_str("null")?;
                        }
                    }
                    serializer.container_suffix(']', depth, first)?;
                    Ok(true)
                })
            }
            JsValue::Object(object) => {
                let id = ContainerId::Object(Rc::as_ptr(object) as usize);
                self.with_container(id, |serializer| {
                    let entries = object.try_borrow().map_err(|_| {
                        type_error("JSON.stringify cannot read a mutably borrowed object")
                    })?.entries_exact();
                    serializer.push_char('{')?;
                    let mut first = true;
                    for (key, value) in entries {
                        if serializer.property_list.is_some_and(|properties|
                            !properties.contains(&key)) {
                            continue;
                        }
                        let value = serializer.replaced_value(&key, &value)?;
                        if matches!(value, JsValue::Undefined) {
                            continue;
                        }
                        serializer.count_member()?;
                        serializer.member_prefix(depth, &mut first)?;
                        serializer.push_quoted(&key)?;
                        serializer.push_str(if serializer.indent.is_empty() {
                            ":"
                        } else {
                            ": "
                        })?;
                        if !serializer.serialize_value(&value, depth + 1)? {
                            return Err(type_error(
                                "JSON object member unexpectedly had no serialized value",
                            ).into());
                        }
                    }
                    serializer.container_suffix('}', depth, first)?;
                    Ok(true)
                })
            }
            JsValue::JsonProjection(_) => Err(type_error(
                "A selected toJSON projection returned another unresolved JSON projection",
            ).into()),
        }
    }

    fn with_container(
        &mut self,
        id: ContainerId,
        serialize: impl FnOnce(&mut Self) -> Result<bool, E>,
    ) -> Result<bool, E> {
        if !self.active.insert(id) {
            return Err(type_error("Converting circular structure to JSON").into());
        }
        let result = serialize(self);
        self.active.remove(&id);
        result
    }

    fn count_node(&mut self, depth: usize) -> Result<(), E> {
        if depth > self.limits.max_depth {
            return Err(range_error(
                "JSON nesting exceeds the configured depth limit",
            ).into());
        }
        self.nodes = self
            .nodes
            .checked_add(1)
            .ok_or_else(|| range_error("JSON node count overflow"))?;
        if self.nodes > self.limits.max_nodes {
            return Err(range_error("JSON value exceeds the configured node limit").into());
        }
        Ok(())
    }

    fn count_member(&mut self) -> Result<(), E> {
        self.members = self
            .members
            .checked_add(1)
            .ok_or_else(|| range_error("JSON member count overflow"))?;
        if self.members > self.limits.max_members {
            return Err(range_error(
                "JSON value exceeds the configured member limit",
            ).into());
        }
        Ok(())
    }

    fn member_prefix(&mut self, depth: usize, first: &mut bool) -> Result<(), E> {
        if !*first {
            self.push_char(',')?;
        }
        if !self.indent.is_empty() {
            self.push_char('\n')?;
            self.push_indent(depth + 1)?;
        }
        *first = false;
        Ok(())
    }

    fn container_suffix(&mut self, close: char, depth: usize, empty: bool) -> Result<(), E> {
        if !empty && !self.indent.is_empty() {
            self.push_char('\n')?;
            self.push_indent(depth)?;
        }
        self.push_char(close)
    }

    fn push_indent(&mut self, depth: usize) -> Result<(), E> {
        let indent = self.indent.clone();
        for _ in 0..depth {
            self.push_js_string(&indent)?;
        }
        Ok(())
    }

    fn push_quoted(&mut self, value: &JsString) -> Result<(), E> {
        self.push_char('"')?;
        let units = value.units();
        let mut index = 0;
        while index < units.len() {
            let unit = units[index];
            match unit {
                value if value == u16::from(b'"') => self.push_str("\\\"")?,
                value if value == u16::from(b'\\') => self.push_str("\\\\")?,
                0x0008 => self.push_str("\\b")?,
                0x000c => self.push_str("\\f")?,
                0x000a => self.push_str("\\n")?,
                0x000d => self.push_str("\\r")?,
                0x0009 => self.push_str("\\t")?,
                0x0000..=0x001f => self.push_str(&format!("\\u{unit:04x}"))?,
                0xd800..=0xdbff if matches!(units.get(index + 1), Some(0xdc00..=0xdfff)) => {
                    self.push_units(&units[index..index + 2])?;
                    index += 1;
                }
                0xd800..=0xdfff => self.push_str(&format!("\\u{unit:04x}"))?,
                _ => self.push_units(&[unit])?,
            }
            index += 1;
        }
        self.push_char('"')
    }

    fn push_char(&mut self, value: char) -> Result<(), E> {
        let mut encoded = [0_u16; 2];
        self.push_units(value.encode_utf16(&mut encoded))
    }

    fn push_str(&mut self, value: &str) -> Result<(), E> {
        self.push_units(&value.encode_utf16().collect::<Vec<_>>())
    }

    fn push_js_string(&mut self, value: &JsString) -> Result<(), E> {
        self.push_units(value.units())
    }

    fn push_units(&mut self, value: &[u16]) -> Result<(), E> {
        let added_bytes = utf8_length(value)
            .ok_or_else(|| type_error("JSON serialization attempted to publish invalid UTF-16"))?;
        let next_bytes = self
            .output_bytes
            .checked_add(added_bytes)
            .ok_or_else(|| range_error("JSON output length overflow"))?;
        if next_bytes > self.limits.max_output_bytes {
            return Err(range_error("JSON output exceeds the configured byte limit").into());
        }
        self.output
            .try_reserve(value.len())
            .map_err(|_| range_error("JSON output allocation failed"))?;
        self.output.extend_from_slice(value);
        self.output_bytes = next_bytes;
        Ok(())
    }
}

fn utf8_length(units: &[u16]) -> Option<usize> {
    let mut length = 0_usize;
    let mut index = 0_usize;
    while index < units.len() {
        let unit = units[index];
        let bytes = match unit {
            0x0000..=0x007f => 1,
            0x0080..=0x07ff => 2,
            0xd800..=0xdbff => {
                if !matches!(units.get(index + 1), Some(0xdc00..=0xdfff)) {
                    return None;
                }
                index += 1;
                4
            }
            0xdc00..=0xdfff => return None,
            _ => 3,
        };
        length = length.checked_add(bytes)?;
        index += 1;
    }
    Some(length)
}

fn json_number(value: f64) -> String {
    if !value.is_finite() {
        return "null".to_string();
    }
    if value == 0.0 {
        return "0".to_string();
    }
    let absolute = value.abs();
    if !(1e-6..1e21).contains(&absolute) {
        return normalize_exponential(format!("{value:e}"));
    }
    let text = value.to_string();
    if text.contains(['e', 'E']) {
        expand_exponential(&text)
    } else {
        text
    }
}

fn normalize_exponential(value: String) -> String {
    let Some((mantissa, exponent)) = value.split_once('e') else {
        return value;
    };
    let exponent = exponent.parse::<i32>().unwrap_or(0);
    if exponent >= 0 {
        format!("{mantissa}e+{exponent}")
    } else {
        format!("{mantissa}e{exponent}")
    }
}

fn expand_exponential(value: &str) -> String {
    let Some((mantissa, exponent)) = value.split_once('e').or_else(|| value.split_once('E')) else {
        return value.to_string();
    };
    let exponent = exponent.parse::<i32>().unwrap_or(0);
    let negative = mantissa.starts_with('-');
    let unsigned = mantissa.trim_start_matches('-');
    let decimal = unsigned.find('.').unwrap_or(unsigned.len());
    let digits = unsigned.replace('.', "");
    let point = i32::try_from(decimal).unwrap_or(i32::MAX) + exponent;
    let mut expanded = if point <= 0 {
        format!("0.{}{}", "0".repeat((-point) as usize), digits)
    } else if usize::try_from(point).unwrap_or(usize::MAX) >= digits.len() {
        format!("{}{}", digits, "0".repeat(point as usize - digits.len()))
    } else {
        let point = point as usize;
        format!("{}.{}", &digits[..point], &digits[point..])
    };
    if negative {
        expanded.insert(0, '-');
    }
    expanded
}

struct Parser<'a> {
    input: &'a [u16],
    pos: usize,
    limits: JsonLimits,
    nodes: usize,
    members: usize,
}

impl<'a> Parser<'a> {
    fn new(input: &'a JsString, limits: JsonLimits) -> Self {
        Self {
            input: input.units(),
            pos: 0,
            limits,
            nodes: 0,
            members: 0,
        }
    }

    fn parse_value(&mut self, depth: usize) -> JsResult<JsValue> {
        self.count_node(depth)?;
        self.skip_ws();
        match self.peek() {
            Some(0x006e) => self.parse_literal(&[0x006e, 0x0075, 0x006c, 0x006c], JsValue::Null),
            Some(0x0074) => {
                self.parse_literal(&[0x0074, 0x0072, 0x0075, 0x0065], JsValue::Bool(true))
            }
            Some(0x0066) => self.parse_literal(
                &[0x0066, 0x0061, 0x006c, 0x0073, 0x0065],
                JsValue::Bool(false),
            ),
            Some(0x0022) => self.parse_string().map(JsValue::String),
            Some(0x005b) => self.parse_array(depth),
            Some(0x007b) => self.parse_object(depth),
            Some(0x002d | 0x0030..=0x0039) => self.parse_number().map(JsValue::Number),
            _ => Err(syntax_error("JSON.parse expected a value")),
        }
    }

    fn count_node(&mut self, depth: usize) -> JsResult<()> {
        if depth > self.limits.max_depth {
            return Err(range_error(
                "JSON nesting exceeds the configured depth limit",
            ));
        }
        self.nodes = self
            .nodes
            .checked_add(1)
            .ok_or_else(|| range_error("JSON node count overflow"))?;
        if self.nodes > self.limits.max_nodes {
            return Err(range_error("JSON input exceeds the configured node limit"));
        }
        Ok(())
    }

    fn count_member(&mut self) -> JsResult<()> {
        self.members = self
            .members
            .checked_add(1)
            .ok_or_else(|| range_error("JSON member count overflow"))?;
        if self.members > self.limits.max_members {
            return Err(range_error(
                "JSON input exceeds the configured member limit",
            ));
        }
        Ok(())
    }

    fn parse_literal(&mut self, literal: &[u16], value: JsValue) -> JsResult<JsValue> {
        if self.input.get(self.pos..self.pos + literal.len()) == Some(literal) {
            self.pos += literal.len();
            Ok(value)
        } else {
            Err(syntax_error("JSON.parse invalid literal"))
        }
    }

    fn parse_string(&mut self) -> JsResult<JsString> {
        self.expect(0x0022)?;
        let mut out = Vec::new();
        while let Some(unit) = self.next() {
            match unit {
                0x0022 => return Ok(JsString::from_units(out)),
                0x005c => self.parse_escape(&mut out)?,
                0x00..=0x1f => return Err(syntax_error("JSON string contains control character")),
                _ => out.push(unit),
            }
        }
        Err(syntax_error("unterminated JSON string"))
    }

    fn parse_escape(&mut self, out: &mut Vec<u16>) -> JsResult<()> {
        let unit = match self.next() {
            Some(0x0022) => 0x0022,
            Some(0x005c) => 0x005c,
            Some(0x002f) => 0x002f,
            Some(0x0062) => 0x0008,
            Some(0x0066) => 0x000c,
            Some(0x006e) => 0x000a,
            Some(0x0072) => 0x000d,
            Some(0x0074) => 0x0009,
            Some(0x0075) => return self.parse_unicode_escape(out),
            _ => return Err(syntax_error("invalid JSON string escape")),
        };
        out.push(unit);
        Ok(())
    }

    fn parse_unicode_escape(&mut self, out: &mut Vec<u16>) -> JsResult<()> {
        out.push(self.parse_hex_unit()?);
        Ok(())
    }

    fn parse_hex_unit(&mut self) -> JsResult<u16> {
        let mut value = 0_u16;
        for _ in 0..4 {
            let unit = self
                .next()
                .ok_or_else(|| syntax_error("unterminated unicode escape"))?;
            value = value
                .checked_mul(16)
                .and_then(|current| hex(unit).map(|digit| current + u16::from(digit)))
                .ok_or_else(|| syntax_error("invalid unicode escape"))?;
        }
        Ok(value)
    }

    fn parse_number(&mut self) -> JsResult<f64> {
        let start = self.pos;
        if self.peek() == Some(0x002d) {
            self.pos += 1;
        }
        match self.peek() {
            Some(0x0030) => {
                self.pos += 1;
                if matches!(self.peek(), Some(0x0030..=0x0039)) {
                    return Err(syntax_error("invalid JSON number"));
                }
            }
            Some(0x0031..=0x0039) => self.consume_digits(),
            _ => return Err(syntax_error("invalid JSON number")),
        }
        if self.peek() == Some(0x002e) {
            self.pos += 1;
            if !matches!(self.peek(), Some(0x0030..=0x0039)) {
                return Err(syntax_error("invalid JSON number"));
            }
            self.consume_digits();
        }
        if matches!(self.peek(), Some(0x0065 | 0x0045)) {
            self.pos += 1;
            if matches!(self.peek(), Some(0x002b | 0x002d)) {
                self.pos += 1;
            }
            if !matches!(self.peek(), Some(0x0030..=0x0039)) {
                return Err(syntax_error("invalid JSON number"));
            }
            self.consume_digits();
        }
        String::from_utf16(&self.input[start..self.pos])
            .ok()
            .and_then(|text| text.parse::<f64>().ok())
            .ok_or_else(|| syntax_error("invalid JSON number"))
    }

    fn parse_array(&mut self, depth: usize) -> JsResult<JsValue> {
        self.expect(0x005b)?;
        let mut values = Vec::new();
        self.skip_ws();
        if self.peek() == Some(0x005d) {
            self.pos += 1;
            return Ok(JsValue::from(values));
        }
        loop {
            self.count_member()?;
            values.push(self.parse_value(depth + 1)?);
            self.skip_ws();
            match self.next() {
                Some(0x002c) => {}
                Some(0x005d) => return Ok(JsValue::from(values)),
                _ => return Err(syntax_error("JSON array expected comma or close bracket")),
            }
        }
    }

    fn parse_object(&mut self, depth: usize) -> JsResult<JsValue> {
        self.expect(0x007b)?;
        let mut object = JsObject::new();
        self.skip_ws();
        if self.peek() == Some(0x007d) {
            self.pos += 1;
            return Ok(JsValue::object(object));
        }
        loop {
            self.count_member()?;
            self.skip_ws();
            let key = self.parse_string()?;
            self.skip_ws();
            self.expect(0x003a)?;
            object.set_exact(key, self.parse_value(depth + 1)?);
            self.skip_ws();
            match self.next() {
                Some(0x002c) => {}
                Some(0x007d) => return Ok(JsValue::object(object)),
                _ => return Err(syntax_error("JSON object expected comma or close brace")),
            }
        }
    }

    fn consume_digits(&mut self) {
        while matches!(self.peek(), Some(0x0030..=0x0039)) {
            self.pos += 1;
        }
    }

    fn expect(&mut self, expected: u16) -> JsResult<()> {
        match self.next() {
            Some(actual) if actual == expected => Ok(()),
            _ => Err(syntax_error("JSON.parse unexpected token")),
        }
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(0x0020 | 0x000a | 0x000d | 0x0009)) {
            self.pos += 1;
        }
    }

    fn next(&mut self) -> Option<u16> {
        let unit = self.peek()?;
        self.pos += 1;
        Some(unit)
    }

    fn peek(&self) -> Option<u16> {
        self.input.get(self.pos).copied()
    }

    fn is_done(&self) -> bool {
        self.pos == self.input.len()
    }
}

fn hex(unit: u16) -> Option<u8> {
    match unit {
        0x0030..=0x0039 => Some((unit - 0x0030) as u8),
        0x0061..=0x0066 => Some((unit - 0x0061 + 10) as u8),
        0x0041..=0x0046 => Some((unit - 0x0041 + 10) as u8),
        _ => None,
    }
}
