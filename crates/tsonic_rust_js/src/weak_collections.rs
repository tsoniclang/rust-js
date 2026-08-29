use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use tsonic_rust_runtime::{Null, ObjectIdentity, ObjectIdentityCarrier, WeakObjectIdentity};

use crate::array::JsArray;
use crate::equality::{hash_identity, JsHash, JsSameValueZero, JsStrictEqual};

struct WeakMapEntry<V> {
    identity: WeakObjectIdentity,
    value: V,
}

struct WeakMapState<V> {
    entries: HashMap<usize, WeakMapEntry<V>>,
}

pub struct JsWeakMap<K: ObjectIdentityCarrier, V> {
    state: Rc<RefCell<WeakMapState<V>>>,
    key: std::marker::PhantomData<fn(K)>,
    identity: ObjectIdentity,
}

impl<K: ObjectIdentityCarrier, V> Clone for JsWeakMap<K, V> {
    fn clone(&self) -> Self {
        Self {
            state: Rc::clone(&self.state),
            key: std::marker::PhantomData,
            identity: self.identity.clone(),
        }
    }
}

impl<K: ObjectIdentityCarrier, V> JsWeakMap<K, V> {
    pub fn new() -> Self {
        Self {
            state: Rc::new(RefCell::new(WeakMapState {
                entries: HashMap::new(),
            })),
            key: std::marker::PhantomData,
            identity: ObjectIdentity::new(),
        }
    }

    pub fn from_entries(entries: impl IntoIterator<Item = (K, V)>) -> Self {
        let result = Self::new();
        for (key, value) in entries {
            result.set_discard(key, value);
        }
        result
    }

    pub fn from_null(_: Null) -> Self {
        Self::new()
    }

    pub fn from_array(entries: &JsArray<(K, V)>) -> Self
    where
        K: Clone,
        V: Clone,
    {
        Self::from_entries(entries.iter_values())
    }

    pub fn get(&self, key: &K) -> Option<V>
    where
        V: Clone,
    {
        self.remove_dead();
        let identity = key.object_identity();
        self.state
            .borrow()
            .entries
            .get(&identity.key())
            .filter(|entry| entry.identity.matches(identity))
            .map(|entry| entry.value.clone())
    }

    pub fn has(&self, key: &K) -> bool {
        self.remove_dead();
        let identity = key.object_identity();
        self.state
            .borrow()
            .entries
            .get(&identity.key())
            .is_some_and(|entry| entry.identity.matches(identity))
    }

    pub fn set(&self, key: K, value: V) -> Self {
        self.set_discard(key, value);
        self.clone()
    }

    pub fn set_discard(&self, key: K, value: V) {
        self.remove_dead();
        let identity = key.object_identity();
        self.state.borrow_mut().entries.insert(
            identity.key(),
            WeakMapEntry {
                identity: identity.downgrade(),
                value,
            },
        );
    }

    pub fn delete(&self, key: &K) -> bool {
        self.remove_dead();
        let identity = key.object_identity();
        let key = identity.key();
        let mut state = self.state.borrow_mut();
        if !state
            .entries
            .get(&key)
            .is_some_and(|entry| entry.identity.matches(identity))
        {
            return false;
        }
        state.entries.remove(&key);
        true
    }

    fn remove_dead(&self) {
        self.state
            .borrow_mut()
            .entries
            .retain(|_, entry| entry.identity.is_alive());
    }
}

impl<K: ObjectIdentityCarrier, V> Default for JsWeakMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: ObjectIdentityCarrier, V> ObjectIdentityCarrier for JsWeakMap<K, V> {
    fn object_identity(&self) -> &ObjectIdentity {
        &self.identity
    }
}

impl<K: ObjectIdentityCarrier, V> PartialEq for JsWeakMap<K, V> {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.state, &other.state)
    }
}

impl<K: ObjectIdentityCarrier, V> Eq for JsWeakMap<K, V> {}

impl<K: ObjectIdentityCarrier, V> JsSameValueZero for JsWeakMap<K, V> {
    fn same_value_zero(&self, other: &Self) -> bool {
        self == other
    }
}

impl<K: ObjectIdentityCarrier, V> JsHash for JsWeakMap<K, V> {
    fn js_hash(&self) -> u64 {
        hash_identity(Rc::as_ptr(&self.state) as usize)
    }
}

impl<K: ObjectIdentityCarrier, V> JsStrictEqual for JsWeakMap<K, V> {
    fn strict_equal(&self, other: &Self) -> bool {
        self == other
    }
}

pub struct JsWeakSet<K: ObjectIdentityCarrier> {
    state: Rc<RefCell<HashMap<usize, WeakObjectIdentity>>>,
    key: std::marker::PhantomData<fn(K)>,
    identity: ObjectIdentity,
}

impl<K: ObjectIdentityCarrier> Clone for JsWeakSet<K> {
    fn clone(&self) -> Self {
        Self {
            state: Rc::clone(&self.state),
            key: std::marker::PhantomData,
            identity: self.identity.clone(),
        }
    }
}

impl<K: ObjectIdentityCarrier> JsWeakSet<K> {
    pub fn new() -> Self {
        Self {
            state: Rc::new(RefCell::new(HashMap::new())),
            key: std::marker::PhantomData,
            identity: ObjectIdentity::new(),
        }
    }

    pub fn from_values(values: impl IntoIterator<Item = K>) -> Self {
        let result = Self::new();
        for value in values {
            result.add_discard(value);
        }
        result
    }

    pub fn from_null(_: Null) -> Self {
        Self::new()
    }

    pub fn from_array(values: &JsArray<K>) -> Self
    where
        K: Clone,
    {
        Self::from_values(values.iter_values())
    }

    pub fn add(&self, value: K) -> Self {
        self.add_discard(value);
        self.clone()
    }

    pub fn add_discard(&self, value: K) {
        self.remove_dead();
        let identity = value.object_identity();
        self.state
            .borrow_mut()
            .insert(identity.key(), identity.downgrade());
    }

    pub fn has(&self, value: &K) -> bool {
        self.remove_dead();
        let identity = value.object_identity();
        self.state
            .borrow()
            .get(&identity.key())
            .is_some_and(|entry| entry.matches(identity))
    }

    pub fn delete(&self, value: &K) -> bool {
        self.remove_dead();
        let identity = value.object_identity();
        let key = identity.key();
        let mut state = self.state.borrow_mut();
        if !state.get(&key).is_some_and(|entry| entry.matches(identity)) {
            return false;
        }
        state.remove(&key);
        true
    }

    fn remove_dead(&self) {
        self.state
            .borrow_mut()
            .retain(|_, identity| identity.is_alive());
    }
}

impl<K: ObjectIdentityCarrier> Default for JsWeakSet<K> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: ObjectIdentityCarrier> ObjectIdentityCarrier for JsWeakSet<K> {
    fn object_identity(&self) -> &ObjectIdentity {
        &self.identity
    }
}

impl<K: ObjectIdentityCarrier> PartialEq for JsWeakSet<K> {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.state, &other.state)
    }
}

impl<K: ObjectIdentityCarrier> Eq for JsWeakSet<K> {}

impl<K: ObjectIdentityCarrier> JsSameValueZero for JsWeakSet<K> {
    fn same_value_zero(&self, other: &Self) -> bool {
        self == other
    }
}

impl<K: ObjectIdentityCarrier> JsHash for JsWeakSet<K> {
    fn js_hash(&self) -> u64 {
        hash_identity(Rc::as_ptr(&self.state) as usize)
    }
}

impl<K: ObjectIdentityCarrier> JsStrictEqual for JsWeakSet<K> {
    fn strict_equal(&self, other: &Self) -> bool {
        self == other
    }
}
