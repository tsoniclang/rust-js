use std::cell::Cell;
use std::rc::Rc;
use tsonic_rust_js::equality::{JsHash, JsSameValueZero};
use tsonic_rust_js::JsMap;
use tsonic_rust_runtime::TsonicError;

#[derive(Clone, Debug)]
struct IdentityKey(std::rc::Rc<()>);

#[derive(Clone)]
struct CountedKey {
    value: u64,
    comparisons: Rc<Cell<usize>>,
}

impl JsHash for CountedKey {
    fn js_hash(&self) -> u64 {
        self.value
    }
}

impl JsSameValueZero for CountedKey {
    fn same_value_zero(&self, other: &Self) -> bool {
        self.comparisons.set(self.comparisons.get() + 1);
        self.value == other.value
    }
}

impl PartialEq for IdentityKey {
    fn eq(&self, other: &Self) -> bool {
        std::rc::Rc::ptr_eq(&self.0, &other.0)
    }
}

#[test]
fn map_preserves_insertion_order_and_updates_existing_key() {
    let map = JsMap::new();
    map.set("a".to_string(), 1);
    map.set("b".to_string(), 2);
    map.set("a".to_string(), 3);

    assert_eq!(map.len(), 2);
    assert_eq!(map.get(&"a".to_string()), Some(3));
    assert_eq!(map.keys(), vec!["a".to_string(), "b".to_string()]);
    assert!(map.delete(&"b".to_string()));
    assert!(!map.has(&"b".to_string()));
}

#[test]
fn map_uses_same_value_zero_for_nan() {
    let map = JsMap::new();
    map.set(f64::NAN, "nan");
    assert_eq!(map.get(&f64::NAN), Some("nan"));
    map.set(-0.0, "zero");
    assert_eq!(map.get(&0.0), Some("zero"));
}

#[test]
fn map_exact_equality_entrypoints_preserve_project_identity() {
    let first = IdentityKey(std::rc::Rc::new(()));
    let alias = first.clone();
    let distinct = IdentityKey(std::rc::Rc::new(()));
    let map = JsMap::new();

    map.set_eq(first, "first");
    assert_eq!(map.get_eq(&alias), Some("first"));
    assert_eq!(map.get_eq(&distinct), None);
    map.set_eq(alias.clone(), "updated");
    assert_eq!(map.len(), 1);
    assert_eq!(map.get_eq(&alias), Some("updated"));
    assert!(!map.delete_eq(&distinct));
    assert!(map.delete_eq(&alias));
}

#[test]
fn string_keys_accept_borrowed_string_lookups() {
    let map = JsMap::from_entries([("name".to_string(), 1)]);
    assert_eq!(map.get("name"), Some(1));
    assert!(map.has("name"));
    assert!(map.delete("name"));
}

#[test]
fn map_iterable_constructor_and_for_each_are_closed() {
    let map = tsonic_rust_js::JsMap::from_entries([(1, "a"), (1, "b"), (2, "c")]);
    assert_eq!(map.len(), 2);
    assert_eq!(map.get(&1), Some("b"));
    let mut seen = Vec::new();
    map.for_each(|value, key, _| seen.push((key, value)));
    assert_eq!(seen, vec![(1, "b"), (2, "c")]);
    assert_eq!(map.entries(), vec![(1, "b"), (2, "c")]);

    let mut callback_count = 0;
    map.for_each_zero(|| callback_count += 1);
    assert_eq!(callback_count, 2);
    let mut values = Vec::new();
    map.for_each_value(|value| values.push(value));
    assert_eq!(values, vec!["b", "c"]);
    let mut pairs = Vec::new();
    map.for_each_value_key(|value, key| pairs.push((key, value)));
    assert_eq!(pairs, vec![(1, "b"), (2, "c")]);
}

#[test]
fn map_aliases_share_state_and_iteration_observes_live_mutation() {
    let map = JsMap::from_entries([(1, "a")]);
    let alias = map.clone();
    alias.set(2, "b");
    assert!(map.ptr_eq(&alias));
    assert_eq!(map.get(&2), Some("b"));

    let mut seen = Vec::new();
    map.for_each(|value, key, current| {
        seen.push((key, value));
        if key == 1 {
            current.set(3, "c");
            current.delete(&2);
        }
    });
    assert_eq!(seen, vec![(1, "a"), (3, "c")]);
}

#[test]
fn fallible_map_callbacks_preserve_arity_live_mutation_and_short_circuiting() {
    let map = JsMap::from_entries([(1, "a"), (2, "b")]);
    let mut zero_visits = 0;
    map.try_for_each_zero(|| {
        zero_visits += 1;
        Ok::<_, TsonicError>(())
    })
    .unwrap();
    assert_eq!(zero_visits, 2);

    let mut values = Vec::new();
    map.try_for_each_value(|value| {
        values.push(value);
        Ok::<_, TsonicError>(())
    })
    .unwrap();
    assert_eq!(values, vec!["a", "b"]);

    let mut pairs = Vec::new();
    map.try_for_each_value_key(|value, key| {
        pairs.push((key, value));
        Ok::<_, TsonicError>(())
    })
    .unwrap();
    assert_eq!(pairs, vec![(1, "a"), (2, "b")]);

    let mut seen = Vec::new();
    let failure = map.try_for_each(|value, key, current| {
        seen.push((key, value));
        if key == 1 {
            current.set(3, "c");
            current.delete(&2);
            Ok(())
        } else {
            Err(TsonicError::unsupported("stop"))
        }
    });
    assert_eq!(failure, Err(TsonicError::unsupported("stop")));
    assert_eq!(seen, vec![(1, "a"), (3, "c")]);
}

#[test]
fn map_indexes_same_value_zero_keys_without_linear_scans() {
    let comparisons = Rc::new(Cell::new(0));
    let map = JsMap::new();
    for value in 0..4096 {
        map.set_discard(
            CountedKey {
                value,
                comparisons: comparisons.clone(),
            },
            value,
        );
    }
    comparisons.set(0);
    assert_eq!(
        map.get(&CountedKey {
            value: 4095,
            comparisons: comparisons.clone()
        }),
        Some(4095),
    );
    assert!(comparisons.get() <= 2);
}

#[test]
fn discard_set_entrypoint_preserves_map_identity_and_contents() {
    let map = JsMap::new();
    let alias = map.clone();
    map.set_discard("name".to_string(), 1);
    assert!(map.ptr_eq(&alias));
    assert_eq!(alias.get("name"), Some(1));
}
