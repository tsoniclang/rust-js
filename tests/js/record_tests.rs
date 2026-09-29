use tsonic_rust_js::object::records;
use tsonic_rust_runtime::Record;

#[test]
fn record_projection_preserves_exact_values_and_reference_identity() {
    let target = Record::from_entries([("first".to_owned(), Some(9_007_199_254_740_993_u64))]);
    assert_eq!(records::keys(&target).get(0), Some("first".to_owned()));
    assert_eq!(
        records::values(&target).get(0),
        Some(Some(9_007_199_254_740_993))
    );
    assert_eq!(
        records::entries(&target).get(0),
        Some(("first".to_owned(), Some(9_007_199_254_740_993)))
    );
    let source = Record::from_entries([("second".to_owned(), None)]);
    let result = records::assign(&target, &source);
    assert_eq!(result, target);
    assert!(result.contains_key(&"second".to_owned()));
    assert_eq!(records::assign(&target, &target), target);
}
