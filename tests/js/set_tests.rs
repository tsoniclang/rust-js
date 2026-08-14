use tsonic_rust_js::JsSet;
use tsonic_rust_runtime::TsonicError;

#[derive(Clone, Debug)]
struct IdentityValue(std::rc::Rc<()>);

impl PartialEq for IdentityValue {
    fn eq(&self, other: &Self) -> bool {
        std::rc::Rc::ptr_eq(&self.0, &other.0)
    }
}

#[test]
fn set_preserves_order_and_uniqueness() {
    let set = JsSet::new();
    set.add("a".to_string());
    set.add("b".to_string());
    set.add("a".to_string());

    assert_eq!(set.len(), 2);
    assert_eq!(set.values(), vec!["a".to_string(), "b".to_string()]);
    assert!(set.delete(&"a".to_string()));
    assert!(!set.has(&"a".to_string()));
}

#[test]
fn set_uses_same_value_zero_for_nan() {
    let set = JsSet::new();
    set.add(f64::NAN);
    set.add(f64::NAN);
    assert_eq!(set.len(), 1);
    assert!(set.has(&f64::NAN));
}

#[test]
fn set_constructors_and_exact_equality_preserve_source_contracts() {
    let array = tsonic_rust_js::JsArray::from_dense(vec![1, 2, 1]);
    assert_eq!(JsSet::from_array(&array).values(), vec![1, 2]);
    assert_eq!(JsSet::from_fixed_array(&[2, 1, 2]).values(), vec![2, 1]);

    let first = IdentityValue(std::rc::Rc::new(()));
    let alias = first.clone();
    let distinct = IdentityValue(std::rc::Rc::new(()));
    let set = JsSet::new();
    set.add_eq(first)
        .add_eq(alias.clone())
        .add_eq(distinct.clone());
    assert_eq!(set.len(), 2);
    assert!(set.has_eq(&alias));
    assert!(set.delete_eq(&alias));
    assert!(!set.delete_eq(&alias));
    assert!(set.has_eq(&distinct));
}

#[test]
fn string_values_accept_borrowed_string_lookups() {
    let set = JsSet::from_values(["name".to_string()]);
    assert!(set.has("name"));
    assert!(set.delete("name"));
}

#[test]
fn set_iterable_constructor_and_for_each_are_closed() {
    let set = tsonic_rust_js::JsSet::from_values([1, 1, 2]);
    assert_eq!(set.len(), 2);
    let mut seen = Vec::new();
    set.for_each(|value, _, _| seen.push(value));
    assert_eq!(seen, vec![1, 2]);

    let mut callback_count = 0;
    set.for_each_zero(|| callback_count += 1);
    assert_eq!(callback_count, 2);
    let mut values = Vec::new();
    set.for_each_value(|value| values.push(value));
    assert_eq!(values, vec![1, 2]);
    let mut pairs = Vec::new();
    set.for_each_value_key(|value, key| pairs.push((key, value)));
    assert_eq!(pairs, vec![(1, 1), (2, 2)]);
}

#[test]
fn set_algebra_preserves_insertion_order() {
    let left = JsSet::from_values([1, 2, 3, 4]);
    let right = JsSet::from_values([3, 5, 1]);

    // union: receiver order first, then the other's unseen values.
    assert_eq!(
        left.union(&right).values(),
        vec![1, 2, 3, 4, 5],
        "union order"
    );
    assert_eq!(right.union(&left).values(), vec![3, 5, 1, 2, 4]);

    // intersection/difference follow the receiver's order.
    assert_eq!(left.intersection(&right).values(), vec![1, 3]);
    assert_eq!(right.intersection(&left).values(), vec![3, 1]);
    assert_eq!(left.difference(&right).values(), vec![2, 4]);
    assert_eq!(right.difference(&left).values(), vec![5]);

    // symmetricDifference: receiver-only values, then the other's.
    assert_eq!(left.symmetric_difference(&right).values(), vec![2, 4, 5]);
    assert_eq!(right.symmetric_difference(&left).values(), vec![5, 2, 4]);
}

#[test]
fn set_aliases_share_state_and_iteration_observes_live_mutation() {
    let set = JsSet::from_values([1]);
    let alias = set.clone();
    alias.add(2);
    assert!(set.ptr_eq(&alias));
    assert!(set.has(&2));

    let mut seen = Vec::new();
    set.for_each(|value, _, current| {
        seen.push(value);
        if value == 1 {
            current.add(3);
            current.delete(&2);
        }
    });
    assert_eq!(seen, vec![1, 3]);
}

#[test]
fn fallible_set_callbacks_preserve_arity_live_mutation_and_short_circuiting() {
    let set = JsSet::from_values([1, 2]);
    let mut zero_visits = 0;
    set.try_for_each_zero(|| {
        zero_visits += 1;
        Ok::<_, TsonicError>(())
    })
    .unwrap();
    assert_eq!(zero_visits, 2);

    let mut values = Vec::new();
    set.try_for_each_value(|value| {
        values.push(value);
        Ok::<_, TsonicError>(())
    })
    .unwrap();
    assert_eq!(values, vec![1, 2]);

    let mut pairs = Vec::new();
    set.try_for_each_value_key(|value, key| {
        pairs.push((key, value));
        Ok::<_, TsonicError>(())
    })
    .unwrap();
    assert_eq!(pairs, vec![(1, 1), (2, 2)]);

    let mut seen = Vec::new();
    let failure = set.try_for_each(|value, _, current| {
        seen.push(value);
        if value == 1 {
            current.add(3);
            current.delete(&2);
            Ok(())
        } else {
            Err(TsonicError::unsupported("stop"))
        }
    });
    assert_eq!(failure, Err(TsonicError::unsupported("stop")));
    assert_eq!(seen, vec![1, 3]);
}

#[test]
fn set_algebra_predicates() {
    let small = JsSet::from_values([1, 2]);
    let big = JsSet::from_values([3, 2, 1]);
    let other = JsSet::from_values([4, 5]);
    let empty: JsSet<i32> = JsSet::new();

    assert!(small.is_subset_of(&big));
    assert!(!big.is_subset_of(&small));
    assert!(big.is_superset_of(&small));
    assert!(!small.is_superset_of(&big));
    assert!(small.is_disjoint_from(&other));
    assert!(!small.is_disjoint_from(&big));

    // The empty set is a subset of everything and disjoint from everything.
    assert!(empty.is_subset_of(&small));
    assert!(small.is_superset_of(&empty));
    assert!(empty.is_disjoint_from(&empty));
    assert!(small.is_subset_of(&small) && small.is_superset_of(&small));
}

#[test]
fn set_algebra_uses_same_value_zero() {
    let with_nan = JsSet::from_values([f64::NAN, 1.0]);
    let other_nan = JsSet::from_values([f64::NAN]);
    assert_eq!(with_nan.intersection(&other_nan).len(), 1);
    assert_eq!(with_nan.union(&other_nan).len(), 2);
    assert_eq!(with_nan.difference(&other_nan).len(), 1);
    assert_eq!(with_nan.symmetric_difference(&other_nan).len(), 1);
    assert!(other_nan.is_subset_of(&with_nan));
    assert!(!other_nan.is_disjoint_from(&with_nan));

    // SameValueZero: 0 and -0 are the same member.
    let zero = JsSet::from_values([0.0_f64]);
    let negative_zero = JsSet::from_values([-0.0_f64]);
    assert!(zero.is_subset_of(&negative_zero));
    assert_eq!(zero.union(&negative_zero).len(), 1);
}
