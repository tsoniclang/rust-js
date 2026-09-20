//! Canonical JavaScript array carrier and static helpers.

pub mod construction;
mod entries;
mod fallible_callbacks;
pub mod js_array;
mod locations;
pub mod number_array_like;
pub mod statics;

pub use construction::{construct_length, ArrayLength};
pub use entries::JsArrayEntries;
pub use js_array::{JsArray, JsArrayIterator};
pub use statics::{
    from_dense_array, from_string, from_string_map, from_string_map_with_index,
    from_string_map_zero, from_string_try_map, from_string_try_map_with_index,
    from_string_try_map_zero, from_vec, from_vec_map, from_vec_map_with_index, from_vec_map_zero,
    from_vec_try_map, from_vec_try_map_with_index, from_vec_try_map_zero, is_array, is_array_value,
    of, JsArrayConcatItem,
};
