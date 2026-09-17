use std::cell::Cell;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;
use tsonic_rust_js::JsArray;
use tsonic_rust_runtime::Location;

#[test]
fn array_locations_preserve_aliases_keys_and_live_storage() {
    let values = JsArray::from_dense(vec![3, 4]);
    let alias = values.clone();
    let first = values.element_location(0.0);
    let same = alias.element_location(-0.0);
    assert!(Location::same(Some(&first), Some(&same)));
    assert_eq!(Location::hash(Some(&first)), Location::hash(Some(&same)));
    assert!(!Location::same(
        Some(&first),
        Some(&values.element_location(1.0))
    ));
    assert!(!Location::same(
        Some(&first),
        Some(&JsArray::from_dense(vec![3]).element_location(0.0))
    ));
    for value in 0..1024 {
        values.push(value);
    }
    first.store(7);
    assert_eq!(alias.get(0), Some(7));
    alias.set(0, 9);
    assert_eq!(same.load(), 9);
    values.delete_at(0);
    assert!(catch_unwind(AssertUnwindSafe(|| first.load())).is_err());
    first.store(11);
    assert_eq!(same.load(), 11);
    values.set_number(-1.0, 13);
    let negative = values.element_location(-1.0);
    assert_eq!(negative.load(), 13);
    negative.store(17);
    assert_eq!(alias.get_number(-1.0), Some(17));
    assert!(Location::same(
        Some(&negative),
        Some(&alias.element_location(-1.0))
    ));
    values.set_number(f64::NAN, 19);
    assert_eq!(values.element_location(f64::NAN).load(), 19);
    assert!(Location::same(
        Some(&values.element_location(f64::NAN)),
        Some(&alias.element_location(f64::NAN))
    ));
    let absent = JsArray::from_dense(vec![None::<i32>]);
    assert_eq!(absent.element_location(0.0).load(), None);
}

#[test]
fn array_locations_retain_owners_without_retargeting_rebound_variables() {
    struct Value(Rc<Cell<bool>>);
    impl Drop for Value {
        fn drop(&mut self) {
            self.0.set(false);
        }
    }
    let alive = Rc::new(Cell::new(true));
    let mut values = JsArray::from_dense(vec![Rc::new(Value(Rc::clone(&alive)))]);
    let pointer = values.element_location(0.0);
    values = JsArray::new();
    assert!(values.is_empty());
    assert!(alive.get());
    assert!(pointer.load().0.get());
    drop(pointer);
    assert!(!alive.get());
}
