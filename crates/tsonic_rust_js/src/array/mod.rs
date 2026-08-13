//! Canonical JavaScript array carrier and static helpers.

mod fallible_callbacks;
pub mod js_array;
pub mod slot;
pub mod statics;

pub use js_array::{JsArray, JsArrayIterator};
pub use slot::JsSlot;
pub use statics::{from_string, is_array, is_array_value, of, JsArrayConcatItem};
