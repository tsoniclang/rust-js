use std::future::Future;
use std::pin::pin;
use std::task::{Context, Poll, Waker};

use tsonic_rust_js::{
    json, promise_all_settled, promise_any, promise_race, ArrayBuffer, IntlCollator,
    IntlDateTimeFormat, IntlNumberFormat, JsArray, JsObject, JsPromise, JsString, JsSymbol,
    JsValue, JsWeakMap, JsWeakSet, PromiseSettledResult,
};
use tsonic_rust_runtime::{Callable, TsonicError};

#[test]
fn symbols_preserve_fresh_and_registry_identity() {
    let first = JsSymbol::create_string("state");
    let second = JsSymbol::create_string("state");
    let registered = JsSymbol::for_key("state");

    assert_ne!(first, second);
    assert_eq!(registered, JsSymbol::for_key("state"));
    assert_eq!(JsSymbol::key_for(&first), None);
    assert_eq!(JsSymbol::key_for(&registered).as_deref(), Some("state"));
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
}

#[test]
fn promise_combinators_preserve_settlement_and_finally_behavior() {
    let fulfilled = JsPromise::resolved(7);
    let rejected = JsPromise::rejected(TsonicError::unsupported("no"));
    let values = JsArray::from_dense(vec![rejected.clone(), fulfilled.clone()]);

    assert!(block_on(promise_race(&values).await_result()).is_err());
    assert_eq!(block_on(promise_any(&values).await_result()).unwrap(), 7);

    let settled = block_on(promise_all_settled(&values).await_value());
    assert!(matches!(settled[0], PromiseSettledResult::Rejected(_)));
    assert!(matches!(settled[1], PromiseSettledResult::Fulfilled(ref value) if value.value == 7));

    let finalizer_runs = std::rc::Rc::new(std::cell::Cell::new(0));
    let count = std::rc::Rc::clone(&finalizer_runs);
    let finalized = fulfilled.finally(Callable::new(move |()| count.set(count.get() + 1)));
    assert_eq!(block_on(finalized.await_result()).unwrap(), 7);
    assert_eq!(finalizer_runs.get(), 1);
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
    assert_eq!(date.resolved_options().time_zone(), "UTC");
    assert!(IntlDateTimeFormat::with_locale("fr-FR").is_err());

    let number_options = JsValue::object(JsObject::from_pairs([
        ("style", string_value("percent")),
        ("maximumFractionDigits", JsValue::Number(1.0)),
    ]));
    let number = IntlNumberFormat::with_locale_options("en-US", &number_options).unwrap();
    assert_eq!(number.format(0.125), "12.5%");

    let collator_options = JsValue::object(JsObject::from_pairs([("numeric", JsValue::Bool(true))]));
    let collator = IntlCollator::with_locale_options("en-US", &collator_options).unwrap();
    assert!(collator.compare("item2", "item10") < 0.0);
}

#[test]
fn json_replacer_and_property_list_traverse_only_closed_values() {
    let source = JsValue::object(JsObject::from_pairs([
        ("keep", JsValue::Number(1.0)),
        ("drop", JsValue::Number(2.0)),
    ]));
    let replaced = json::stringify_with_replacer(&source, |key, value| {
        if key == "drop" { JsValue::Undefined } else { value }
    })
    .unwrap();
    assert_eq!(replaced.as_deref(), Some("{\"keep\":1}"));

    let properties = JsValue::array(JsArray::from_dense(vec![string_value("drop")]));
    let selected = json::stringify_with_property_list_and_space_number(&source, &properties, 2.0)
        .unwrap();
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
