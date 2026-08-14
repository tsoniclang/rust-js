//! Canonical JavaScript array carrier and static helpers.

mod fallible_callbacks;
pub mod js_array;
pub mod slot;
pub mod statics;

pub use js_array::{JsArray, JsArrayIterator};
pub use slot::JsSlot;
pub use statics::{
    from_string, from_vec, from_vec_map, from_vec_map_with_index, from_vec_map_zero,
    from_vec_try_map, from_vec_try_map_with_index, from_vec_try_map_zero, is_array, is_array_value,
    of, JsArrayConcatItem,
};
