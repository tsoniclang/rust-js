//! JS-facing closed APIs.

#![forbid(unsafe_code)]

pub mod abi;
pub mod array;
pub mod array_buffer;
pub mod boolean;
mod coercion;
pub mod console;
pub mod data_view;
pub mod date;
pub mod equality;
pub mod errors;
pub mod exact_string;
pub mod globals;
pub mod intl;
pub mod js_string;
pub mod json;
pub mod map;
pub mod math;
pub mod number;
pub mod object;
pub mod promise;
pub mod regexp;
pub mod set;
pub mod string;
pub mod symbol;
pub mod timers;
pub mod typed_array;
pub mod uri;
pub mod value;
pub mod weak_collections;
pub mod web;
pub mod wrappers;

pub use array::{JsArray, JsSlot};
pub use array_buffer::ArrayBuffer;
pub use data_view::DataView;
pub use errors::{
    aggregate_error, eval_error, range_error, reference_error, syntax_error, type_error,
    unsupported, uri_error, JsResult,
};
pub use intl::{
    IntlCollator, IntlDateTimeFormat, IntlDateTimeFormatPart, IntlNumberFormat,
    IntlNumberFormatPart, IntlResolvedCollatorOptions, IntlResolvedDateTimeFormatOptions,
    IntlResolvedNumberFormatOptions,
};
pub use js_string::JsString;
pub use map::JsMap;
pub use object::{JsObject, JsPropertyValue};
pub use promise::{JsPromise, PromiseFulfilledResult, PromiseRejectedResult, PromiseSettledResult};
pub use set::JsSet;
pub use symbol::JsSymbol;
pub use tsonic_rust_runtime::{JsError, JsErrorKind};
pub use typed_array::{
    Float32Array, Float64Array, Int16Array, Int32Array, Int8Array, TypedArrayKind, Uint16Array,
    Uint32Array, Uint8Array, Uint8ClampedArray,
};
pub use value::{JsClosedValue, JsClosedValueCarrier, JsValue};
pub use weak_collections::{JsWeakMap, JsWeakSet};
pub use web::{
    AbortController, AbortSignal, AddEventListenerOptions, Blob, BlobPart, Body, CustomEvent,
    DomException, Event, EventInit, EventListenerOptions, EventTarget, File, FormData,
    FormDataValue, Headers, ImportMeta, Navigator, Request, Response, Storage,
};
