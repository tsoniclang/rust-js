use tsonic_rust_js::{array::JsArray, exact_string, numeric::IndexInput, string, JsString};

#[test]
fn character_constructors_preserve_native_integers_and_explicit_encodings() {
    assert_eq!(string::from_code_point(&[0x1f600_u32]).unwrap(), "😀");
    assert_eq!(string::from_char_code(&[65_u8, 66]).unwrap(), "AB");
    assert_eq!(string::from_code_point::<usize>(&[]).unwrap(), "");
    assert!(string::from_code_point(&[9_007_199_254_740_993_u64]).is_err());
    assert!(string::from_code_point(&[u128::MAX]).is_err());
    assert!(string::from_code_point(&[-1_i64]).is_err());
    assert!(string::from_code_point(&[0xd800_u32]).is_err());
    assert_eq!(
        exact_string::from_char_code(&[9_007_199_254_740_993_u64]),
        JsString::from_utf8("\u{1}")
    );
    assert_eq!(
        exact_string::from_char_code(&[u128::MAX]),
        JsString::from_units(vec![0xffff])
    );
    assert_eq!(
        exact_string::from_code_point(&[0xd800_u32]).unwrap(),
        JsString::from_units(vec![0xd800])
    );
    assert!(exact_string::from_code_point(&[u64::MAX]).is_err());
    assert_eq!(
        exact_string::from_code_point(&[-0.0_f64]).unwrap(),
        JsString::from_utf8("\0")
    );
    for value in [0.5, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(string::from_code_point(&[value]).is_err());
        assert!(exact_string::from_code_point(&[value]).is_err());
    }
}

#[test]
fn integer_indices_preserve_storage_aliases_and_do_not_narrow() {
    let values = JsArray::from_dense(vec![1, 2, 3]);
    let alias = values.clone();
    values.set_number(1_usize, 5);
    assert_eq!(alias.get_number(1_u64), Some(5));
    assert_eq!(*alias.borrow_number_element(1_u128).unwrap(), 5);
    assert_eq!(alias.at(-1_i64), Some(3));
    assert_eq!(alias.index_of(&5, 0_usize), 1);
    assert_eq!(alias.last_index_of(&5, u64::MAX), 1);
    assert_eq!(alias.slice_to(1_u64, 3_usize).values(), vec![5, 3]);
    values.fill_to(7, 1_u64, 3_usize);
    assert_eq!(alias.values(), vec![1, 7, 7]);
    values.copy_within_to(0_u64, 1_usize, 3_u32);
    assert_eq!(alias.values(), vec![7, 7, 7]);
    assert_eq!(values.splice_many(1_u64, 1_usize, [9]).values(), vec![7]);
    assert_eq!(alias.values(), vec![7, 9, 7]);
    for index in [u64::from(u32::MAX), 9_007_199_254_740_993, u64::MAX - 1] {
        assert_eq!(values.get_number(index), None);
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(
            || values.set_number(index, 0)
        ))
        .is_err());
        assert_eq!(alias.values(), vec![7, 9, 7]);
        assert!(!JsArray::contains_number_property(index, &values));
    }
    values.set_number(-9_007_199_254_740_993_i64, 17);
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(
        || values.set_number(u128::MAX, 0)
    ))
    .is_err());
    assert_eq!(values.get_number(u128::MAX), None);
    assert_eq!(values.get_number(-9_007_199_254_740_993_i64), Some(17));
    assert_eq!(values.get_number(-9_007_199_254_740_992_i64), None);
    let pointer = values.element_location(0_usize);
    pointer.store(11);
    assert_eq!(alias.get_number(0_usize), Some(11));
}

#[test]
fn string_apis_accept_native_indices_in_both_explicit_encodings() {
    let native = "ab😀cd";
    let exact = JsString::from_utf8(native);
    assert_eq!(string::at(native, -1_isize).unwrap().as_deref(), Some("d"));
    assert_eq!(string::char_at(native, 2_usize).unwrap(), "😀");
    assert_eq!(string::slice_to(native, 2_usize, 6_u64).unwrap(), "😀");
    assert_eq!(string::substring(native, 6_usize, 2_u64).unwrap(), "😀");
    assert_eq!(string::substr(native, 2_usize, 4_u64).unwrap(), "😀");
    assert_eq!(string::index_of(native, "d", 6_usize), 7);
    assert_eq!(string::last_index_of(native, "b", usize::MAX), 1);
    assert_eq!(string::repeat("ab", 3_usize).unwrap(), "ababab");
    assert_eq!(string::pad_start_with("ab", 4_usize, "x").unwrap(), "xxab");
    assert_eq!(
        string::split(native, "😀", 2_usize).unwrap().values(),
        vec!["ab", "cd"]
    );
    assert_eq!(exact_string::char_code_at(&exact, 2_usize), 0xd83d as f64);
    assert_eq!(exact_string::code_point_at(&exact, 2_u64), Some(0x1f600));
    assert_eq!(
        exact_string::slice_to(&exact, 2_usize, 4_u64),
        JsString::from_utf8("😀")
    );
    assert_eq!(
        exact_string::repeat(&JsString::from_utf8("ab"), 3_usize).unwrap(),
        JsString::from_utf8("ababab")
    );
    assert_eq!(string::char_at(native, u64::MAX).unwrap(), "");
    assert_eq!(string::repeat("", u128::MAX).unwrap(), "");
    assert!(string::repeat("x", u128::MAX).is_err());
}

#[test]
fn integer_index_arithmetic_has_exact_wide_bounds_without_allocation() {
    let length = usize::MAX;
    assert_eq!(u128::MAX.last_index(length), Some(length - 1));
    assert_eq!((-1_i64).last_index(length), Some(length - 1));
    assert_eq!(i128::MIN.last_index(length), None);
    assert_eq!(f64::NEG_INFINITY.last_index(length), None);
    assert_eq!(f64::INFINITY.last_index(length), Some(length - 1));
    assert_eq!((-0.9_f64).last_index(length), Some(0));
    assert_eq!(f64::NAN.last_index(length), Some(0));
    assert_eq!((-1_i64).positive_index(length), 0);
    assert_eq!(u128::MAX.positive_index(length), length);
    assert_eq!(1_usize.last_index(0), None);
}
