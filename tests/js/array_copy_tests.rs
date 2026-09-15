use std::cell::Cell;
use std::rc::Rc;
use tsonic_rust_js::abi::{
    array_from_dense_array, array_from_optional_array, array_from_undefined_array, JsArray,
};
use tsonic_rust_runtime::Undefined;

#[test]
fn copies_preserve_element_identity_and_create_independent_array_storage() {
    let object = Rc::new(Cell::new(3));
    let original = JsArray::from_dense(vec![Rc::clone(&object)]);
    let copy = array_from_dense_array(&original);
    assert!(!original.ptr_eq(&copy));
    assert!(Rc::ptr_eq(&original.get(0).unwrap(), &copy.get(0).unwrap()));
    object.set(7);
    assert_eq!(copy.get(0).unwrap().get(), 7);
    copy.delete_at(0);
    assert!(original.has_index(0));
}

#[test]
fn sparse_copies_create_present_undefined_without_mutating_the_source() {
    let source = JsArray::from_sparse(3, vec![(0, Some(4)), (2, None)]);
    let copy = array_from_optional_array(&source);
    assert_eq!(copy.len(), 3);
    assert!(!source.has_index(1));
    assert!(copy.has_index(1));
    assert_eq!(copy.get(0), Some(Some(4)));
    assert_eq!(copy.get(1), Some(None));
    assert_eq!(copy.get(2), Some(None));
    let undefined = JsArray::<Undefined>::with_length(2);
    let present = array_from_undefined_array(&undefined);
    assert!(present.has_index(0) && present.has_index(1));
    assert!(present.get(0).is_some());
    assert!(!undefined.has_index(0));
    assert!(array_from_optional_array(&JsArray::<Option<i32>>::new()).is_empty());
}

#[test]
#[should_panic(expected = "checked array density invariant violated")]
fn dense_copy_defends_its_compiler_proved_presence_invariant() {
    array_from_dense_array(&JsArray::<i32>::with_length(1));
}
