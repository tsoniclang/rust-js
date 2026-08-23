//! JavaScript Boolean primitive operations.

use crate::JsString;

pub fn to_string(value: bool) -> JsString {
    if value { "true" } else { "false" }.into()
}

pub fn value_of(value: bool) -> bool {
    value
}
