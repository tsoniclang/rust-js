use tsonic_rust_runtime::{ObjectIdentity, ObjectIdentityCarrier, TsonicResult};

use crate::equality::{ JsHash, JsSameValueZero, JsStrictEqual};

#[derive(Debug)]
pub struct JsSet<T> {
    entries: crate::map::JsMap<T, ()>,
}

impl<T> Clone for JsSet<T> {
    fn clone(&self) -> Self {
        Self { entries: self.entries.clone() }
    }
}

impl<T> PartialEq for JsSet<T> {
    fn eq(&self, other: &Self) -> bool {
        self.ptr_eq(other)
    }
}

impl<T> Eq for JsSet<T> {}

impl<T> JsSameValueZero for JsSet<T> {
    fn same_value_zero(&self, other: &Self) -> bool {
        self.ptr_eq(other)
    }
}

impl<T> JsHash for JsSet<T> {
    fn js_hash(&self) -> u64 {
        self.entries.js_hash()
    }
}

impl<T> JsStrictEqual for JsSet<T> {
    fn strict_equal(&self, other: &Self) -> bool {
        self.ptr_eq(other)
    }
}

impl<T> JsSet<T> {
    pub fn new() -> Self {
        Self { entries: crate::map::JsMap::new() }
    }

    pub fn from_values(values: impl IntoIterator<Item = T>) -> Self
    where
        T: JsHash + JsSameValueZero,
    {
        let set = Self::new();
        for value in values {
            set.add(value);
        }
        set
    }

    pub fn from_array(values: &crate::array::JsArray<T>) -> Self
    where
        T: Clone + JsHash + JsSameValueZero,
    {
        Self::from_values(values.iter_values())
    }

    pub fn from_fixed_array<const LENGTH: usize>(values: &[T; LENGTH]) -> Self
    where
        T: Clone + JsHash + JsSameValueZero,
    {
        Self::from_values(values.iter().cloned())
    }

    pub fn ptr_eq(&self, other: &Self) -> bool {
        self.entries.ptr_eq(&other.entries)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn clear(&self) {
        self.entries.clear();
    }

    pub fn has<Q>(&self, value: &Q) -> bool
    where
        T: JsSameValueZero<Q>,
        Q: JsHash + ?Sized,
    {
        self.entries.has(value)
    }

    pub fn has_eq(&self, value: &T) -> bool
    where
        T: PartialEq,
    {
        self.entries.has_eq(value)
    }

    pub fn add(&self, value: T) -> Self
    where
        T: JsHash + JsSameValueZero,
    {
        self.add_discard(value);
        self.clone()
    }

    pub fn add_discard(&self, value: T)
    where
        T: JsHash + JsSameValueZero,
    {
        self.entries.set_discard(value, ());
    }

    pub fn add_eq(&self, value: T) -> Self
    where
        T: PartialEq,
    {
        self.add_eq_discard(value);
        self.clone()
    }

    pub fn add_eq_discard(&self, value: T)
    where
        T: PartialEq,
    {
        self.entries.set_eq_discard(value, ());
    }

    pub fn delete<Q>(&self, value: &Q) -> bool
    where
        T: JsSameValueZero<Q>,
        Q: JsHash + ?Sized,
    {
        self.entries.delete(value)
    }

    pub fn delete_eq(&self, value: &T) -> bool
    where
        T: PartialEq,
    {
        self.entries.delete_eq(value)
    }

    pub fn keys(&self) -> Vec<T>
    where
        T: Clone,
    {
        self.values()
    }

    pub fn values(&self) -> Vec<T>
    where
        T: Clone,
    {
        self.entries.keys()
    }

    pub fn entries(&self) -> Vec<(T, T)>
    where
        T: Clone,
    {
        self.values()
            .into_iter()
            .map(|value| (value.clone(), value))
            .collect()
    }

    pub fn for_each_zero<F>(&self, mut callback: F)
    where
        T: Clone,
        F: FnMut(),
    {
        self.for_each(|_, _, _| callback());
    }

    pub fn for_each_value<F>(&self, mut callback: F)
    where
        T: Clone,
        F: FnMut(T),
    {
        self.for_each(|value, _, _| callback(value));
    }

    pub fn for_each_value_key<F>(&self, mut callback: F)
    where
        T: Clone,
        F: FnMut(T, T),
    {
        self.for_each(|value, key, _| callback(value, key));
    }

    pub fn for_each<F>(&self, mut callback: F)
    where
        T: Clone,
        F: FnMut(T, T, Self),
    {
        self.entries.for_each(|(), value, _| callback(value.clone(), value, self.clone()));
    }

    pub fn difference(&self, other: &Self) -> Self
    where
        T: Clone + JsHash + JsSameValueZero,
    {
        Self::from_values(self.values().into_iter().filter(|value| !other.has(value)))
    }

    pub fn intersection(&self, other: &Self) -> Self
    where
        T: Clone + JsHash + JsSameValueZero,
    {
        Self::from_values(self.values().into_iter().filter(|value| other.has(value)))
    }

    pub fn union(&self, other: &Self) -> Self
    where
        T: Clone + JsHash + JsSameValueZero,
    {
        let output = Self::from_values(self.values());
        for value in other.values() {
            output.add(value);
        }
        output
    }

    pub fn symmetric_difference(&self, other: &Self) -> Self
    where
        T: Clone + JsHash + JsSameValueZero,
    {
        let output = self.difference(other);
        for value in other.values() {
            if !self.has(&value) {
                output.add(value);
            }
        }
        output
    }

    pub fn is_subset_of(&self, other: &Self) -> bool
    where
        T: Clone + JsHash + JsSameValueZero,
    {
        self.values().iter().all(|value| other.has(value))
    }

    pub fn is_superset_of(&self, other: &Self) -> bool
    where
        T: Clone + JsHash + JsSameValueZero,
    {
        other.is_subset_of(self)
    }

    pub fn is_disjoint_from(&self, other: &Self) -> bool
    where
        T: Clone + JsHash + JsSameValueZero,
    {
        self.values().iter().all(|value| !other.has(value))
    }

    fn try_for_each_with<F>(&self, mut callback: F) -> TsonicResult<()>
    where
        T: Clone,
        F: FnMut(T, T, Self) -> TsonicResult<()>,
    {
        self.entries.try_for_each(|(), value, _| callback(value.clone(), value, self.clone()))
    }

    pub fn try_for_each_zero<F>(&self, mut callback: F) -> TsonicResult<()>
    where
        T: Clone,
        F: FnMut() -> TsonicResult<()>,
    {
        self.try_for_each_with(|_, _, _| callback())
    }

    pub fn try_for_each_value<F>(&self, mut callback: F) -> TsonicResult<()>
    where
        T: Clone,
        F: FnMut(T) -> TsonicResult<()>,
    {
        self.try_for_each_with(|value, _, _| callback(value))
    }

    pub fn try_for_each_value_key<F>(&self, mut callback: F) -> TsonicResult<()>
    where
        T: Clone,
        F: FnMut(T, T) -> TsonicResult<()>,
    {
        self.try_for_each_with(|value, key, _| callback(value, key))
    }

    pub fn try_for_each<F>(&self, callback: F) -> TsonicResult<()>
    where
        T: Clone,
        F: FnMut(T, T, Self) -> TsonicResult<()>,
    {
        self.try_for_each_with(callback)
    }
}


impl<T> ObjectIdentityCarrier for JsSet<T> {
    fn object_identity(&self) -> &ObjectIdentity {
        self.entries.object_identity()
    }
}

impl<T> Default for JsSet<T> {
    fn default() -> Self {
        Self::new()
    }
}
