//! Array static constructors and brand checks.

#[derive(Debug)]
pub enum JsArrayConcatItem<T> {
    Value(T),
    Array(super::JsArray<T>),
}

/// Closed `Array.from` conversion for strings.
///
/// This keeps a closed behavior with no iterator callbacks.
/// Code-point semantics are used (`.chars()`), not UTF-16 unit splitting.
pub fn from_string(value: &str) -> super::JsArray<String> {
    super::JsArray::from_dense(value.chars().map(|ch| ch.to_string()).collect())
}

pub fn from_vec<T: Clone>(values: &Vec<T>) -> super::JsArray<T> {
    super::JsArray::from_dense(values.clone())
}

pub fn from_vec_map_zero<T: Clone, U, F>(values: &Vec<T>, mut callback: F) -> super::JsArray<U>
where
    F: FnMut() -> U,
{
    super::JsArray::from_dense(values.iter().map(|_| callback()).collect())
}

pub fn from_vec_map<T: Clone, U, F>(values: &Vec<T>, mut callback: F) -> super::JsArray<U>
where
    F: FnMut(T) -> U,
{
    super::JsArray::from_dense(values.iter().cloned().map(&mut callback).collect())
}

pub fn from_vec_map_with_index<T: Clone, U, F>(
    values: &Vec<T>,
    mut callback: F,
) -> super::JsArray<U>
where
    F: FnMut(T, f64) -> U,
{
    super::JsArray::from_dense(
        values
            .iter()
            .cloned()
            .enumerate()
            .map(|(index, value)| callback(value, index as f64))
            .collect(),
    )
}

pub fn from_vec_try_map_zero<T: Clone, U, E, F>(
    values: &Vec<T>,
    mut callback: F,
) -> Result<super::JsArray<U>, E>
where
    F: FnMut() -> Result<U, E>,
{
    let values = values
        .iter()
        .map(|_| callback())
        .collect::<Result<Vec<_>, E>>()?;
    Ok(super::JsArray::from_dense(values))
}

pub fn from_vec_try_map<T: Clone, U, E, F>(
    values: &Vec<T>,
    mut callback: F,
) -> Result<super::JsArray<U>, E>
where
    F: FnMut(T) -> Result<U, E>,
{
    let values = values
        .iter()
        .cloned()
        .map(&mut callback)
        .collect::<Result<Vec<_>, E>>()?;
    Ok(super::JsArray::from_dense(values))
}

pub fn from_vec_try_map_with_index<T: Clone, U, E, F>(
    values: &Vec<T>,
    mut callback: F,
) -> Result<super::JsArray<U>, E>
where
    F: FnMut(T, f64) -> Result<U, E>,
{
    let values = values
        .iter()
        .cloned()
        .enumerate()
        .map(|(index, value)| callback(value, index as f64))
        .collect::<Result<Vec<_>, E>>()?;
    Ok(super::JsArray::from_dense(values))
}

pub fn of<T, const N: usize>(items: [T; N]) -> super::JsArray<T> {
    super::JsArray::from_dense(Vec::from(items))
}

/// Compile-time array identity helper for statically typed carriers.
pub fn is_array<T>(value: &T) -> bool
where
    T: ArrayBrand + ?Sized,
{
    value.is_array_brand()
}

pub fn is_array_value(value: &crate::value::JsValue) -> bool {
    matches!(value, crate::value::JsValue::Array(_))
}

/// Marker trait for known array-like carrier values.
pub trait ArrayBrand {
    fn is_array_brand(&self) -> bool;
}

impl<T> ArrayBrand for Vec<T> {
    fn is_array_brand(&self) -> bool {
        true
    }
}

impl<T> ArrayBrand for [T] {
    fn is_array_brand(&self) -> bool {
        true
    }
}

impl<T> ArrayBrand for super::JsArray<T> {
    fn is_array_brand(&self) -> bool {
        true
    }
}

impl ArrayBrand for i32 {
    fn is_array_brand(&self) -> bool {
        false
    }
}
