use std::future::Future;
use std::pin::pin;
use std::task::{Context, Poll, Waker};

use tsonic_rust_js::abi::{promise_all_settled, promise_any, promise_race};
use tsonic_rust_js::{
    json, ArrayBuffer, IntlCollator, IntlDateTimeFormat, IntlNumberFormat, JsArray, JsObject,
    JsPromise, JsString, JsSymbol, JsValue, JsWeakMap, JsWeakSet, PromiseSettledResult,
};
use tsonic_rust_runtime::{Callable, Null, TsonicError, Undefined};

#[test]
fn intl_number_precision_grouping_and_exact_integers() {
    let make = |pairs: Vec<(&str, JsValue)>| {
        IntlNumberFormat::with_locale_options("en", &JsValue::object(JsObject::from_pairs(pairs)))
            .unwrap()
    };
    let significant = make(vec![("maximumSignificantDigits", JsValue::Number(3.0))]);
    let resolved = significant.resolved_options();
    assert_eq!(resolved.minimum_fraction_digits(), None);
    assert_eq!(resolved.maximum_fraction_digits(), None);
    assert_eq!(resolved.minimum_significant_digits(), Some(1.0));
    assert_eq!(resolved.maximum_significant_digits(), Some(3.0));
    assert_eq!(resolved.use_grouping().as_string(), "auto");
    assert_eq!(significant.format(1234.5), "1,230");
    assert_eq!(significant.format(0.0012345), "0.00123");
    let mixed = make(vec![
        ("maximumSignificantDigits", JsValue::Number(3.9)),
        ("maximumFractionDigits", JsValue::Number(-1.0)),
    ]);
    assert_eq!(
        mixed.resolved_options().maximum_significant_digits(),
        Some(3.0)
    );
    assert_eq!(mixed.resolved_options().maximum_fraction_digits(), None);
    assert_eq!(mixed.format(1234.5), "1,230");
    let default = IntlNumberFormat::new();
    assert_eq!(
        default.resolved_options().minimum_fraction_digits(),
        Some(0.0)
    );
    assert_eq!(
        default.resolved_options().maximum_fraction_digits(),
        Some(3.0)
    );
    assert_eq!(
        default.resolved_options().maximum_significant_digits(),
        None
    );
    assert_eq!(default.format(-0.0), "-0");
    assert_eq!(
        make(vec![("roundingIncrement", JsValue::Number(1.9))]).format(1.25),
        "1.25"
    );
    assert_eq!(
        make(vec![("maximumFractionDigits", JsValue::Number(2.0))]).format(1.005),
        "1.01"
    );
    assert_eq!(
        make(vec![("minimumSignificantDigits", JsValue::Number(3.0))]).format(0.0),
        "0.00"
    );
    for (input, strategy, rendered) in [
        (JsValue::Bool(false), None, "1234"),
        (JsValue::Bool(true), Some("always"), "1,234"),
        (string_value("auto"), Some("auto"), "1,234"),
        (string_value("always"), Some("always"), "1,234"),
        (string_value("min2"), Some("min2"), "1234"),
    ] {
        let formatter = make(vec![("useGrouping", input)]);
        let selected = formatter.resolved_options().use_grouping();
        if let Some(strategy) = strategy {
            assert_eq!(selected.type_of(), "string");
            assert_eq!(selected.as_string(), strategy);
        } else {
            assert_eq!(selected.type_of(), "boolean");
            assert!(!selected.as_bool());
        }
        assert_eq!(formatter.format(1234.0), rendered);
        assert_eq!(
            formatter.format(12345.0),
            if strategy.is_none() {
                "12345"
            } else {
                "12,345"
            }
        );
    }
    for (value, expected) in [
        (9_007_199_254_740_993_i64, "9,007,199,254,740,993"),
        (i64::MIN, "-9,223,372,036,854,775,808"),
    ] {
        assert_eq!(default.format(value), expected);
        let parts = default
            .format_to_parts(value)
            .values()
            .into_iter()
            .flatten()
            .map(|part| part.value())
            .collect::<Vec<_>>()
            .concat();
        assert_eq!(parts, expected);
        assert_eq!(
            tsonic_rust_js::abi::integer_to_locale_string(value),
            expected
        );
    }
    assert_eq!(default.format(u64::MAX), "18,446,744,073,709,551,615");
    let plain = make(vec![("useGrouping", JsValue::Bool(false))]);
    assert_eq!(
        plain.format(i128::MIN),
        "-170141183460469231731687303715884105728"
    );
    assert_eq!(
        plain.format(u128::MAX),
        "340282366920938463463374607431768211455"
    );
    let arbitrary = tsonic_rust_runtime::BigInt::from_decimal_literal(
        "340282366920938463463374607431768211456123",
    );
    assert_eq!(
        plain.format(&arbitrary),
        "340282366920938463463374607431768211456123"
    );
    assert_eq!(
        plain.format(&-arbitrary),
        "-340282366920938463463374607431768211456123"
    );
    assert_eq!(
        make(vec![("style", string_value("percent"))]).format(9_007_199_254_740_993_i64),
        "900,719,925,474,099,300%"
    );
}

#[test]
fn intl_number_rejects_unsupported_options_instead_of_ignoring_them() {
    for (name, value) in [
        ("useGrouping", string_value("unknown")),
        ("notation", string_value("compact")),
        ("notation", string_value("scientific")),
        ("style", string_value("unit")),
        ("unit", string_value("not-a-unit")),
        ("unit", string_value("meter")),
        ("currencySign", string_value("accounting")),
        ("roundingMode", string_value("halfEven")),
        ("roundingPriority", string_value("morePrecision")),
        ("roundingIncrement", JsValue::Number(5.0)),
        ("signDisplay", string_value("always")),
        ("trailingZeroDisplay", string_value("stripIfInteger")),
        ("maximumSignificantDigits", JsValue::Number(22.0)),
        ("maximumFractionDigits", JsValue::Number(101.0)),
    ] {
        let result = IntlNumberFormat::with_locale_options(
            "en",
            &JsValue::object(JsObject::from_pairs([(name, value)])),
        );
        assert!(result.is_err(), "option {name} must reject");
    }
}

#[test]
fn symbols_preserve_fresh_and_registry_identity() {
    let empty = JsSymbol::create();
    let numeric = JsSymbol::create_number(42.0);
    let first = JsSymbol::create_string("state");
    let second = JsSymbol::create_string("state");
    let registered = JsSymbol::for_key("state");

    assert_ne!(first, second);
    assert_eq!(registered, JsSymbol::for_key("state"));
    assert_eq!(JsSymbol::key_for(&first), None);
    assert_eq!(JsSymbol::key_for(&registered).as_deref(), Some("state"));
    assert_eq!(empty.description(), None);
    assert_eq!(numeric.description().as_deref(), Some("42"));

    let symbol_value = JsValue::symbol(registered.clone());
    assert_eq!(symbol_value.as_symbol(), Some(&registered));
    assert_eq!(symbol_value.reference_identity_key(), None);
    let object_value = JsValue::object(JsObject::from_pairs([("value", JsValue::Number(1.0))]));
    assert!(object_value.reference_identity_key().is_some());
}

#[test]
fn nullish_runtime_values_project_exactly_to_js_values() {
    assert!(matches!(JsValue::from(Null), JsValue::Null));
    assert!(matches!(JsValue::from(Undefined), JsValue::Undefined));
}

#[test]
fn weak_collections_use_exact_object_identity() {
    let first = ArrayBuffer::new(1.0).unwrap();
    let alias = first.clone();
    let second = ArrayBuffer::new(1.0).unwrap();
    let map = JsWeakMap::new();
    let set = JsWeakSet::new();

    map.set_discard(first.clone(), 7);
    set.add_discard(first);
    assert_eq!(map.get(&alias), Some(7));
    assert!(set.has(&alias));
    assert_eq!(map.get(&second), None);
    assert!(!set.has(&second));
    assert!(map.delete(&alias));
    assert!(set.delete(&alias));

    let empty_map = JsWeakMap::<ArrayBuffer, i32>::from_null(Null);
    let empty_set = JsWeakSet::<ArrayBuffer>::from_null(Null);
    assert!(!empty_map.has(&second));
    assert!(!empty_set.has(&second));
}

#[test]
fn promise_combinators_preserve_settlement_and_finally_behavior() {
    let fulfilled = JsPromise::resolved(7);
    let rejected = JsPromise::rejected(TsonicError::unsupported("no"));
    let values = JsArray::from_dense(vec![rejected.clone(), fulfilled.clone()]);

    assert!(block_on(promise_race(&values).await_result()).is_err());
    assert_eq!(block_on(promise_any(&values).await_result()).unwrap(), 7);

    let settled = block_on(promise_all_settled(&values).await_value());
    assert!(matches!(
        settled.get(0),
        Some(PromiseSettledResult::Rejected(_))
    ));
    assert!(matches!(
        settled.get(1),
        Some(PromiseSettledResult::Fulfilled(value)) if value.value == 7
    ));

    let finalizer_runs = std::rc::Rc::new(std::cell::Cell::new(0));
    let count = std::rc::Rc::clone(&finalizer_runs);
    let finalized = fulfilled.finally(Callable::new(move |()| {
        count.set(count.get() + 1);
        Ok(())
    }));
    assert_eq!(block_on(finalized.await_result()).unwrap(), 7);
    assert_eq!(finalizer_runs.get(), 1);

    assert_eq!(
        block_on(fulfilled.finally_default().await_result()).unwrap(),
        7
    );
    let failed_finalizer = fulfilled.finally(Callable::new(|()| {
        Err(TsonicError::unsupported("finalizer"))
    }));
    assert!(block_on(failed_finalizer.await_result()).is_err());
}

#[test]
fn intl_is_deterministic_and_rejects_unapproved_locale_data() {
    let date_options = JsValue::object(JsObject::from_pairs([
        ("timeZone", string_value("UTC")),
        ("year", string_value("numeric")),
        ("month", string_value("2-digit")),
        ("day", string_value("2-digit")),
    ]));
    let date = IntlDateTimeFormat::with_locale_options("en-US", &date_options).unwrap();
    assert_eq!(date.format_number(1_686_787_200_000.0), "06/15/2023");
    let date_parts = date.format_to_parts_number(1_686_787_200_000.0);
    let first_date_part = date_parts.get(0).unwrap();
    assert!(!first_date_part.type_value().is_empty());
    assert!(!first_date_part.value().is_empty());
    let date_options = date.resolved_options();
    assert_eq!(date_options.numbering_system(), "latn");
    assert_eq!(date_options.time_zone(), "UTC");
    let locales = JsArray::from_dense(vec!["en-US".to_owned()]);
    let default_date = IntlDateTimeFormat::new();
    assert!(!default_date.format_default().is_empty());
    let selected_date = IntlDateTimeFormat::with_locales(&locales).unwrap();
    assert!(!selected_date.format_to_parts_default().is_empty());
    let instant = tsonic_rust_js::abi::JsDate::from_millis(1_686_787_200_000.0);
    assert!(!selected_date.format_date(&instant).is_empty());
    assert!(!selected_date.format_to_parts_date(&instant).is_empty());
    assert!(IntlDateTimeFormat::with_locales_options(&locales, &JsValue::Undefined,).is_ok());
    assert!(IntlDateTimeFormat::with_locale("fr-FR").is_err());

    let number_options = JsValue::object(JsObject::from_pairs([
        ("style", string_value("percent")),
        ("maximumFractionDigits", JsValue::Number(1.0)),
    ]));
    let number = IntlNumberFormat::with_locale_options("en-US", &number_options).unwrap();
    assert_eq!(number.format(0.125), "12.5%");
    let number_parts = number.format_to_parts(0.125);
    let first_number_part = number_parts.get(0).unwrap();
    assert!(!first_number_part.type_value().is_empty());
    assert!(!first_number_part.value().is_empty());
    let number_options = number.resolved_options();
    assert_eq!(number_options.numbering_system(), "latn");
    assert!(IntlNumberFormat::with_locales(&locales).is_ok());
    assert!(IntlNumberFormat::with_locales_options(&locales, &JsValue::Undefined).is_ok());

    let collator_options =
        JsValue::object(JsObject::from_pairs([("numeric", JsValue::Bool(true))]));
    let collator = IntlCollator::with_locale_options("en-US", &collator_options).unwrap();
    assert!(collator.compare("item2", "item10") < 0.0);
    assert_eq!(collator.resolved_options().collation(), "default");
    assert!(IntlCollator::with_locales(&locales).is_ok());
    assert!(IntlCollator::with_locales_options(&locales, &JsValue::Undefined).is_ok());
}

#[test]
fn timer_inventory_tracks_live_callbacks_exactly() {
    assert!(!tsonic_rust_js::timers::has_timers());
    let timer = tsonic_rust_js::timers::set_timeout_callable(
        Callable::new(|()| Ok::<(), String>(())),
        1_000.0,
    );
    assert!(tsonic_rust_js::timers::has_timers());
    tsonic_rust_js::timers::clear_timeout(timer);
    assert!(!tsonic_rust_js::timers::has_timers());
}

#[test]
fn json_replacer_and_property_list_traverse_only_closed_values() {
    let source = JsValue::object(JsObject::from_pairs([
        ("keep", JsValue::Number(1.0)),
        ("drop", JsValue::Number(2.0)),
    ]));
    let replaced = json::stringify_with_replacer(&source, |key, value| {
        if key == "drop" {
            JsValue::Undefined
        } else {
            value
        }
    })
    .unwrap();
    assert_eq!(replaced.as_deref(), Some("{\"keep\":1}"));

    let properties = JsValue::array(JsArray::from_dense(vec![string_value("drop")]));
    let selected =
        json::stringify_with_property_list_and_space_number(&source, &properties, 2.0).unwrap();
    assert_eq!(selected.as_deref(), Some("{\n  \"drop\": 2\n}"));
}

fn string_value(value: &str) -> JsValue {
    JsValue::String(JsString::from_utf8(value))
}

fn block_on<T>(future: impl Future<Output = T>) -> T {
    let mut future = pin!(future);
    let mut context = Context::from_waker(Waker::noop());
    match future.as_mut().poll(&mut context) {
        Poll::Ready(value) => value,
        Poll::Pending => panic!("capability-closure future unexpectedly remained pending"),
    }
}
