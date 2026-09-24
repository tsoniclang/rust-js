use tsonic_rust_js::JsString;

fn js(value: impl AsRef<str>) -> JsString {
    JsString::from_utf8(value.as_ref())
}

#[path = "js/array_buffer_tests.rs"]
mod array_buffer_tests;
#[path = "js/array_copy_tests.rs"]
mod array_copy_tests;
#[path = "js/array_entries_tests.rs"]
mod array_entries_tests;
#[path = "js/array_location_tests.rs"]
mod array_location_tests;
#[path = "js/atomics_tests.rs"]
mod atomics_tests;
#[path = "js/capability_closure_tests.rs"]
mod capability_closure_tests;
#[path = "js/console_tests.rs"]
mod console_tests;
#[path = "js/construction_tests.rs"]
mod construction_tests;
#[path = "js/date_tests.rs"]
mod date_tests;
#[path = "js/equality_tests.rs"]
mod equality_tests;
#[path = "js/error_value_tests.rs"]
mod error_value_tests;
#[path = "js/fallible_callback_tests.rs"]
mod fallible_callback_tests;
#[path = "js/globals_uri_dataview_tests.rs"]
mod globals_uri_dataview_tests;
#[path = "js/js_array_tests.rs"]
mod js_array_tests;
#[path = "js/json_tests.rs"]
mod json_tests;
#[path = "js/local_date_tests.rs"]
mod local_date_tests;
#[path = "js/map_tests.rs"]
mod map_tests;
#[path = "js/math_tests.rs"]
mod math_tests;
#[path = "js/native_integer_boundaries_tests.rs"]
mod native_integer_boundaries_tests;
#[path = "js/native_performance_tests.rs"]
mod native_performance_tests;
#[path = "js/number_array_like_tests.rs"]
mod number_array_like_tests;
#[path = "js/number_tests.rs"]
mod number_tests;
#[path = "js/numeric_union_tests.rs"]
mod numeric_union_tests;
#[path = "js/object_tests.rs"]
mod object_tests;
#[path = "js/regexp_tests.rs"]
mod regexp_tests;
#[path = "js/set_tests.rs"]
mod set_tests;
#[path = "js/string_number_tests.rs"]
mod string_number_tests;
#[path = "js/string_tests.rs"]
mod string_tests;
#[path = "js/typed_array_tests.rs"]
mod typed_array_tests;
#[path = "js/web_tests.rs"]
mod web_tests;
