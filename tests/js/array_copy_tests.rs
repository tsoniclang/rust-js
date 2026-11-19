use std::cell::Cell;
use std::rc::Rc;
use tsonic_rust_js::abi::{
    array_from_dense_array, array_from_string_map_with_index, array_from_string_try_map,
    array_from_vec_map_with_index, array_from_vec_try_map, JsArray,
};

#[test]
fn consuming_array_values_move_exclusive_storage_and_preserve_shared_aliases() {
    struct Counted {
        copies: Rc<Cell<usize>>,
        value: usize,
    }
    impl Clone for Counted {
        fn clone(&self) -> Self {
            self.copies.set(self.copies.get() + 1);
            Self {
                copies: Rc::clone(&self.copies),
                value: self.value,
            }
        }
    }
    let copies = Rc::new(Cell::new(0));
    let values = vec![Counted {
        copies: Rc::clone(&copies),
        value: 7,
    }];
    let pointer = values.as_ptr();
    let owned = JsArray::from_dense(values).into_values();
    assert_eq!(owned.as_ptr(), pointer);
    assert_eq!(copies.get(), 0);
    assert_eq!(owned[0].value, 7);

    let shared = JsArray::from_dense(owned);
    let alias = shared.clone();
    let copied = shared.into_values();
    assert_ne!(copied.as_ptr(), pointer);
    assert_eq!(copies.get(), 1);
    alias.with_values(|retained| assert_eq!(retained[0].value, 7));
    assert_eq!(copied[0].value, 7);
    assert!(JsArray::<usize>::new().into_values().is_empty());
}

#[test]
fn copies_preserve_element_identity_and_create_independent_array_storage() {
    let object = Rc::new(Cell::new(3));
    let original = JsArray::from_dense(vec![Rc::clone(&object)]);
    let copy = array_from_dense_array(&original);
    assert!(!original.ptr_eq(&copy));
    assert!(Rc::ptr_eq(&original.get(0).unwrap(), &copy.get(0).unwrap()));
    object.set(7);
    assert_eq!(copy.get(0).unwrap().get(), 7);
    copy.set_len(0);
    assert!(original.has_index(0));
}

#[test]
fn dense_copy_clones_each_element_once_without_copying_numeric_properties() {
    struct Counted(Rc<Cell<usize>>);
    impl Clone for Counted {
        fn clone(&self) -> Self {
            self.0.set(self.0.get() + 1);
            Self(Rc::clone(&self.0))
        }
    }
    let copies = Rc::new(Cell::new(0));
    let original = JsArray::from_dense(vec![
        Counted(Rc::clone(&copies)),
        Counted(Rc::clone(&copies)),
    ]);
    original.set_number(-1.0, Counted(Rc::clone(&copies)));
    let copied = array_from_dense_array(&original);
    assert_eq!(copies.get(), 2);
    assert_eq!(copied.len(), 2);
    assert!(copied.get_number(-1.0).is_none());
    assert!(original.get_number(-1.0).is_some());
    assert!(array_from_dense_array(&JsArray::<i32>::new()).is_empty());
}

#[test]
fn optional_copies_retain_explicit_undefined_without_mutating_the_source() {
    let source = JsArray::from_dense(vec![Some(4), None, None]);
    let copy = array_from_dense_array(&source);
    assert_eq!(copy.len(), 3);
    assert!(source.has_index(1));
    assert!(copy.has_index(1));
    assert_eq!(copy.get(0), Some(Some(4)));
    assert_eq!(copy.get(1), Some(None));
    assert_eq!(copy.get(2), Some(None));
    let undefined = JsArray::<()>::with_length(2);
    let present = array_from_dense_array(&undefined);
    assert!(present.has_index(0) && present.has_index(1));
    assert!(present.get(0).is_some());
    assert!(undefined.has_index(0));
    assert!(array_from_dense_array(&JsArray::<Option<i32>>::new()).is_empty());
}

#[test]
fn dense_copy_retains_initialized_native_defaults() {
    assert_eq!(
        array_from_dense_array(&JsArray::<i32>::with_length(1)).get(0),
        Some(0)
    );
}

#[test]
fn mapped_copies_collect_each_requested_value_in_order() {
    let mut visited = Vec::new();
    let copy = array_from_vec_map_with_index(&[4, 7, 9], |value, index| {
        visited.push((value, index));
        value * 2
    });
    assert_eq!(visited, vec![(4, 0), (7, 1), (9, 2)]);
    assert_eq!(copy.values(), vec![8, 14, 18]);
    let text =
        array_from_string_map_with_index("a😀b", |value, index| format!("{index}:{value}"));
    assert_eq!(
        text.values(),
        vec!["0:a".to_owned(), "1:😀".to_owned(), "2:b".to_owned()]
    );
}

#[test]
fn fallible_copies_stop_at_the_exact_error_without_publishing_a_partial_array() {
    let error = Rc::new(Cell::new(1));
    let mut visited = Vec::new();
    let result = array_from_vec_try_map(&[4, 7, 9], |value| {
        visited.push(value);
        if value == 7 {
            Err(Rc::clone(&error))
        } else {
            Ok(value)
        }
    });
    assert_eq!(visited, vec![4, 7]);
    assert!(Rc::ptr_eq(&result.unwrap_err(), &error));
    let mut scalars = Vec::new();
    let result = array_from_string_try_map("a😀b", |value| {
        scalars.push(value.clone());
        if value == "😀" {
            Err(Rc::clone(&error))
        } else {
            Ok(value)
        }
    });
    assert_eq!(scalars, vec!["a", "😀"]);
    assert!(Rc::ptr_eq(&result.unwrap_err(), &error));
}
