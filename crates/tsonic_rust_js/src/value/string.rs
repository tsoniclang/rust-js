use super::{JsClosedValueCarrier, JsClosedValuePayload, JsValue};
use crate::array::JsArrayElement;
use crate::errors::{type_error, unsupported, JsResult};
use crate::string::JsToString;
use std::fmt::Write;

pub fn closed_value_string(value: &JsValue) -> JsResult<String> {
    let mut output = String::new();
    write_value(value, &mut output, None)?;
    Ok(output)
}

struct ArrayPath<'scope> {
    identity: usize,
    parent: Option<&'scope ArrayPath<'scope>>,
}

fn write_value(
    value: &JsValue,
    output: &mut String,
    ancestors: Option<&ArrayPath<'_>>,
) -> JsResult<()> {
    match value {
        JsValue::Null => output.push_str("null"),
        JsValue::Bool(value) => output.push_str(if *value { "true" } else { "false" }),
        JsValue::Number(value) => value.write_js_string(output),
        JsValue::Integer(value) => value.write_js_string(output),
        JsValue::UnsignedInteger(value) => value.write_js_string(output),
        JsValue::Int8(value) => value.write_js_string(output),
        JsValue::Uint8(value) => value.write_js_string(output),
        JsValue::Int16(value) => value.write_js_string(output),
        JsValue::Uint16(value) => value.write_js_string(output),
        JsValue::Int32(value) => value.write_js_string(output),
        JsValue::Uint32(value) => value.write_js_string(output),
        JsValue::NativeInt(value) => value.write_js_string(output),
        JsValue::NativeUint(value) => value.write_js_string(output),
        JsValue::Float32(value) => value.write_js_string(output),
        JsValue::String(value) => output.push_str(value),
        JsValue::Utf16String(value) => output.push_str(&value.to_utf8_lossy()),
        JsValue::Symbol(value) => write!(output, "{value:?}").unwrap(),
        JsValue::Object(_) | JsValue::Record(_) => output.push_str("[object Object]"),
        JsValue::Array(values) => {
            let identity = values.identity();
            let mut previous = ancestors;
            while let Some(ancestor) = previous {
                if ancestor.identity == identity {
                    return Err(type_error(
                        "String conversion does not support cyclic arrays",
                    ));
                }
                previous = ancestor.parent;
            }
            let path = ArrayPath {
                identity,
                parent: ancestors,
            };
            for index in 0..values.len() {
                if index != 0 {
                    output.push(',');
                }
                let mut result = Ok(());
                values.visit_element(index, &mut |element| {
                    result = match element {
                        JsArrayElement::String(value) => {
                            output.push_str(value);
                            Ok(())
                        }
                        JsArrayElement::Value(JsValue::Null) => Ok(()),
                        JsArrayElement::Value(value) => write_value(value, output, Some(&path)),
                    };
                });
                result?;
            }
        }
        JsValue::Closed(value) => match &value.0 {
            JsClosedValuePayload::Empty(value) => value.write_string(output)?,
            JsClosedValuePayload::NativeShared(_) => {
                return Err(unsupported(
                    "Native object erasure does not expose string conversion",
                ));
            }
            JsClosedValuePayload::Object(value) => value.write_string(output)?,
            JsClosedValuePayload::Error(error) => {
                write!(output, "{}", error.kind()).unwrap();
                if !error.message().is_empty() {
                    output.push_str(": ");
                    output.push_str(error.message());
                }
            }
        },
        JsValue::JsonProjection(_) => {
            return Err(unsupported(
                "A JSON-only projection has no native String conversion",
            ))
        }
    }
    Ok(())
}
