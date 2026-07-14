//! Closed JSON parser/stringifier for supported carrier values.

use std::collections::HashSet;
use std::rc::Rc;

use crate::errors::{range_error, syntax_error, type_error, unsupported, JsResult};
use crate::object::JsObject;
use crate::value::JsValue;

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
    let mut parser = Parser::new(text, limits);
    let value = parser.parse_value(0)?;
    parser.skip_ws();
    if parser.is_done() {
        Ok(value)
    } else {
        Err(syntax_error("JSON.parse found trailing input"))
    }
}

pub fn stringify(value: &JsValue) -> JsResult<Option<String>> {
    stringify_with_indent_and_limits(value, "", JsonLimits::default())
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
    if indent.encode_utf16().count() > 10 {
        return Err(type_error(
            "JSON indentation must be pre-resolved to at most 10 UTF-16 code units",
        ));
    }
    let mut serializer = Serializer::new(indent, limits);
    if serializer.serialize_value(value, 0)? {
        Ok(Some(serializer.output))
    } else {
        Ok(None)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum ContainerId {
    Object(usize),
    Array(usize),
}

struct Serializer<'a> {
    indent: &'a str,
    limits: JsonLimits,
    output: String,
    active: HashSet<ContainerId>,
    nodes: usize,
    members: usize,
}

impl<'a> Serializer<'a> {
    fn new(indent: &'a str, limits: JsonLimits) -> Self {
        Self {
            indent,
            limits,
            output: String::new(),
            active: HashSet::new(),
            nodes: 0,
            members: 0,
        }
    }

    fn serialize_value(&mut self, value: &JsValue, depth: usize) -> JsResult<bool> {
        self.count_node(depth)?;
        match value {
            JsValue::Undefined => Ok(false),
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
                let id = ContainerId::Array(Rc::as_ptr(values) as usize);
                self.with_container(id, |serializer| {
                    let values = values.try_borrow().map_err(|_| {
                        type_error("JSON.stringify cannot read a mutably borrowed array")
                    })?;
                    serializer.push_char('[')?;
                    let mut first = true;
                    for value in values.values() {
                        serializer.count_member()?;
                        serializer.member_prefix(depth, &mut first)?;
                        match value {
                            Some(value) if serializer.serialize_value(value, depth + 1)? => {}
                            _ => serializer.push_str("null")?,
                        }
                    }
                    serializer.container_suffix(']', depth, first)?;
                    Ok(true)
                })
            }
            JsValue::Object(object) => {
                let id = ContainerId::Object(Rc::as_ptr(object) as usize);
                self.with_container(id, |serializer| {
                    let object = object.try_borrow().map_err(|_| {
                        type_error("JSON.stringify cannot read a mutably borrowed object")
                    })?;
                    serializer.push_char('{')?;
                    let mut first = true;
                    for (key, value) in object.entries() {
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
                            ));
                        }
                    }
                    serializer.container_suffix('}', depth, first)?;
                    Ok(true)
                })
            }
        }
    }

    fn with_container(
        &mut self,
        id: ContainerId,
        serialize: impl FnOnce(&mut Self) -> JsResult<bool>,
    ) -> JsResult<bool> {
        if !self.active.insert(id) {
            return Err(type_error("Converting circular structure to JSON"));
        }
        let result = serialize(self);
        self.active.remove(&id);
        result
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
            return Err(range_error("JSON value exceeds the configured node limit"));
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
                "JSON value exceeds the configured member limit",
            ));
        }
        Ok(())
    }

    fn member_prefix(&mut self, depth: usize, first: &mut bool) -> JsResult<()> {
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

    fn container_suffix(&mut self, close: char, depth: usize, empty: bool) -> JsResult<()> {
        if !empty && !self.indent.is_empty() {
            self.push_char('\n')?;
            self.push_indent(depth)?;
        }
        self.push_char(close)
    }

    fn push_indent(&mut self, depth: usize) -> JsResult<()> {
        for _ in 0..depth {
            self.push_str(self.indent)?;
        }
        Ok(())
    }

    fn push_quoted(&mut self, value: &str) -> JsResult<()> {
        self.push_char('"')?;
        for ch in value.chars() {
            match ch {
                '"' => self.push_str("\\\"")?,
                '\\' => self.push_str("\\\\")?,
                '\u{0008}' => self.push_str("\\b")?,
                '\u{000c}' => self.push_str("\\f")?,
                '\n' => self.push_str("\\n")?,
                '\r' => self.push_str("\\r")?,
                '\t' => self.push_str("\\t")?,
                ch if (ch as u32) < 0x20 => self.push_str(&format!("\\u{:04x}", ch as u32))?,
                ch => self.push_char(ch)?,
            }
        }
        self.push_char('"')
    }

    fn push_char(&mut self, value: char) -> JsResult<()> {
        let mut encoded = [0_u8; 4];
        self.push_str(value.encode_utf8(&mut encoded))
    }

    fn push_str(&mut self, value: &str) -> JsResult<()> {
        let next = self
            .output
            .len()
            .checked_add(value.len())
            .ok_or_else(|| range_error("JSON output length overflow"))?;
        if next > self.limits.max_output_bytes {
            return Err(range_error("JSON output exceeds the configured byte limit"));
        }
        self.output
            .try_reserve(value.len())
            .map_err(|_| range_error("JSON output allocation failed"))?;
        self.output.push_str(value);
        Ok(())
    }
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
    input: &'a [u8],
    pos: usize,
    limits: JsonLimits,
    nodes: usize,
    members: usize,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str, limits: JsonLimits) -> Self {
        Self {
            input: input.as_bytes(),
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
            Some(b'n') => self.parse_literal(b"null", JsValue::Null),
            Some(b't') => self.parse_literal(b"true", JsValue::Bool(true)),
            Some(b'f') => self.parse_literal(b"false", JsValue::Bool(false)),
            Some(b'"') => self.parse_string().map(JsValue::String),
            Some(b'[') => self.parse_array(depth),
            Some(b'{') => self.parse_object(depth),
            Some(b'-' | b'0'..=b'9') => self.parse_number().map(JsValue::Number),
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

    fn parse_literal(&mut self, literal: &[u8], value: JsValue) -> JsResult<JsValue> {
        if self.input.get(self.pos..self.pos + literal.len()) == Some(literal) {
            self.pos += literal.len();
            Ok(value)
        } else {
            Err(syntax_error("JSON.parse invalid literal"))
        }
    }

    fn parse_string(&mut self) -> JsResult<String> {
        self.expect(b'"')?;
        let mut out = Vec::new();
        while let Some(byte) = self.next() {
            match byte {
                b'"' => {
                    return String::from_utf8(out)
                        .map_err(|_| syntax_error("JSON string contains invalid UTF-8"));
                }
                b'\\' => self.parse_escape(&mut out)?,
                0x00..=0x1f => return Err(syntax_error("JSON string contains control character")),
                _ => out.push(byte),
            }
        }
        Err(syntax_error("unterminated JSON string"))
    }

    fn parse_escape(&mut self, out: &mut Vec<u8>) -> JsResult<()> {
        let ch = match self.next() {
            Some(b'"') => '"',
            Some(b'\\') => '\\',
            Some(b'/') => '/',
            Some(b'b') => '\u{0008}',
            Some(b'f') => '\u{000c}',
            Some(b'n') => '\n',
            Some(b'r') => '\r',
            Some(b't') => '\t',
            Some(b'u') => return self.parse_unicode_escape(out),
            _ => return Err(syntax_error("invalid JSON string escape")),
        };
        let mut buffer = [0_u8; 4];
        out.extend_from_slice(ch.encode_utf8(&mut buffer).as_bytes());
        Ok(())
    }

    fn parse_unicode_escape(&mut self, out: &mut Vec<u8>) -> JsResult<()> {
        let first = self.parse_hex_unit()?;
        let scalar = if (0xd800..=0xdbff).contains(&first) {
            if self.next() != Some(b'\\') || self.next() != Some(b'u') {
                return Err(unsupported(
                    "JSON strings containing lone UTF-16 surrogates require a UTF-16 string carrier",
                ));
            }
            let second = self.parse_hex_unit()?;
            if !(0xdc00..=0xdfff).contains(&second) {
                return Err(unsupported(
                    "JSON strings containing lone UTF-16 surrogates require a UTF-16 string carrier",
                ));
            }
            0x1_0000 + ((u32::from(first) - 0xd800) << 10) + (u32::from(second) - 0xdc00)
        } else if (0xdc00..=0xdfff).contains(&first) {
            return Err(unsupported(
                "JSON strings containing lone UTF-16 surrogates require a UTF-16 string carrier",
            ));
        } else {
            u32::from(first)
        };
        let ch = char::from_u32(scalar).ok_or_else(|| syntax_error("invalid unicode escape"))?;
        let mut buffer = [0_u8; 4];
        out.extend_from_slice(ch.encode_utf8(&mut buffer).as_bytes());
        Ok(())
    }

    fn parse_hex_unit(&mut self) -> JsResult<u16> {
        let mut value = 0_u16;
        for _ in 0..4 {
            let byte = self
                .next()
                .ok_or_else(|| syntax_error("unterminated unicode escape"))?;
            value = value
                .checked_mul(16)
                .and_then(|current| hex(byte).map(|digit| current + u16::from(digit)))
                .ok_or_else(|| syntax_error("invalid unicode escape"))?;
        }
        Ok(value)
    }

    fn parse_number(&mut self) -> JsResult<f64> {
        let start = self.pos;
        if self.peek() == Some(b'-') {
            self.pos += 1;
        }
        match self.peek() {
            Some(b'0') => {
                self.pos += 1;
                if matches!(self.peek(), Some(b'0'..=b'9')) {
                    return Err(syntax_error("invalid JSON number"));
                }
            }
            Some(b'1'..=b'9') => self.consume_digits(),
            _ => return Err(syntax_error("invalid JSON number")),
        }
        if self.peek() == Some(b'.') {
            self.pos += 1;
            if !matches!(self.peek(), Some(b'0'..=b'9')) {
                return Err(syntax_error("invalid JSON number"));
            }
            self.consume_digits();
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.pos += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.pos += 1;
            }
            if !matches!(self.peek(), Some(b'0'..=b'9')) {
                return Err(syntax_error("invalid JSON number"));
            }
            self.consume_digits();
        }
        std::str::from_utf8(&self.input[start..self.pos])
            .ok()
            .and_then(|text| text.parse::<f64>().ok())
            .ok_or_else(|| syntax_error("invalid JSON number"))
    }

    fn parse_array(&mut self, depth: usize) -> JsResult<JsValue> {
        self.expect(b'[')?;
        let mut values = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b']') {
            self.pos += 1;
            return Ok(JsValue::from(values));
        }
        loop {
            self.count_member()?;
            values.push(self.parse_value(depth + 1)?);
            self.skip_ws();
            match self.next() {
                Some(b',') => {}
                Some(b']') => return Ok(JsValue::from(values)),
                _ => return Err(syntax_error("JSON array expected comma or close bracket")),
            }
        }
    }

    fn parse_object(&mut self, depth: usize) -> JsResult<JsValue> {
        self.expect(b'{')?;
        let mut object = JsObject::new();
        self.skip_ws();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Ok(JsValue::object(object));
        }
        loop {
            self.count_member()?;
            self.skip_ws();
            let key = self.parse_string()?;
            self.skip_ws();
            self.expect(b':')?;
            object.set(key, self.parse_value(depth + 1)?);
            self.skip_ws();
            match self.next() {
                Some(b',') => {}
                Some(b'}') => return Ok(JsValue::object(object)),
                _ => return Err(syntax_error("JSON object expected comma or close brace")),
            }
        }
    }

    fn consume_digits(&mut self) {
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.pos += 1;
        }
    }

    fn expect(&mut self, expected: u8) -> JsResult<()> {
        match self.next() {
            Some(actual) if actual == expected => Ok(()),
            _ => Err(syntax_error("JSON.parse unexpected token")),
        }
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\n' | b'\r' | b'\t')) {
            self.pos += 1;
        }
    }

    fn next(&mut self) -> Option<u8> {
        let byte = self.peek()?;
        self.pos += 1;
        Some(byte)
    }

    fn peek(&self) -> Option<u8> {
        self.input.get(self.pos).copied()
    }

    fn is_done(&self) -> bool {
        self.pos == self.input.len()
    }
}

fn hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}
