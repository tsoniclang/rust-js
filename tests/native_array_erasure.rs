use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::hint::black_box;
use std::rc::Rc;
use tsonic_rust_js::array::{JsArrayElement, JsArrayIndex, JsArrayValue};
use tsonic_rust_js::equality::JsStrictEqual;
use tsonic_rust_js::value::{closed_value_string, js_value_from_array};
use tsonic_rust_js::{json, JsArray, JsValue};

struct CountingAllocator;

thread_local! {
    static TRACKED_ALLOCATIONS: Cell<Option<usize>> = const { Cell::new(None) };
    static PROJECTIONS: Cell<usize> = const { Cell::new(0) };
}

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        TRACKED_ALLOCATIONS.with(|count| {
            if let Some(value) = count.get() {
                count.set(Some(value + 1));
            }
        });
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

struct ObservedClone(i32);

impl Clone for ObservedClone {
    fn clone(&self) -> Self {
        PROJECTIONS.with(|count| count.set(count.get() + 1));
        Self(self.0)
    }
}

fn observed_clone(
    array: &JsArrayValue,
    index: JsArrayIndex<'_>,
    visitor: &mut dyn FnMut(JsArrayElement<'_>),
) {
    if let Some(value) = array.get_native_element::<ObservedClone>(index) {
        visitor(JsArrayElement::Value(&JsValue::Int32(value.0)));
    }
}

#[test]
fn owned_reads_execute_native_clone_effects_for_each_selected_read() {
    PROJECTIONS.with(|count| count.set(0));
    let values = JsArray::from_dense(vec![ObservedClone(7)]);
    let erased = JsArrayValue::new(&values, observed_clone);
    assert!(matches!(erased.get_number(0), Some(JsValue::Int32(7))));
    assert!(matches!(erased.get_number(0), Some(JsValue::Int32(7))));
    assert_eq!(PROJECTIONS.with(Cell::get), 2);
}

fn strings(
    array: &JsArrayValue,
    index: JsArrayIndex<'_>,
    visitor: &mut dyn FnMut(JsArrayElement<'_>),
) {
    PROJECTIONS.with(|count| count.set(count.get() + 1));
    array.with_native_element::<String, _>(index, |value| {
        if let Some(value) = value {
            visitor(JsArrayElement::String(value));
        }
    });
}

#[test]
fn typed_erasure_retains_native_backing_and_rejects_invariant_element_layouts() {
    let original = JsArray::from_dense(vec![String::from("first")]);
    PROJECTIONS.with(|count| count.set(0));
    TRACKED_ALLOCATIONS.with(|count| count.set(Some(0)));
    let erased = js_value_from_array(&original, strings);
    let alias = erased.clone();
    for _iteration in 0..10_000 {
        let selected = black_box(&erased).cast_array::<String>().unwrap();
        assert!(selected.ptr_eq(&original));
        assert!(erased.strict_equal(&alias));
        assert_eq!(erased.as_array().unwrap().identity(), original.identity());
        assert_eq!(erased.as_array().unwrap().len(), 1);
        assert!(erased.as_array().unwrap().restore::<JsValue>().is_none());
        assert!(erased.as_array().unwrap().restore::<u64>().is_none());
    }
    let allocations = TRACKED_ALLOCATIONS.with(|count| count.replace(None).unwrap());
    assert_eq!(allocations, 0);
    assert_eq!(PROJECTIONS.with(Cell::get), 0);
    assert!(erased.cast_array::<JsValue>().is_err());
    assert!(JsValue::Bool(true).cast_array::<String>().is_err());
    let restored = erased.cast_array::<String>().unwrap();
    restored.push(String::from("second"));
    assert_eq!(original.len(), 2);
    assert_eq!(alias.as_array().unwrap().len(), 2);
    assert_eq!(closed_value_string(&erased).unwrap(), "first,second");
    assert_eq!(
        json::stringify(&alias).unwrap().unwrap(),
        "[\"first\",\"second\"]"
    );
    if usize::BITS == 64 {
        assert_eq!(std::mem::size_of::<JsValue>(), 40);
    }
}

#[test]
fn erased_indexed_write_checks_real_backing_without_allocating_or_copying() {
    let original = JsArray::from_dense(vec![JsValue::Number(1.0)]);
    let erased = JsValue::array(original.clone());
    let alias = erased.as_array().unwrap();
    TRACKED_ALLOCATIONS.with(|count| count.set(Some(0)));
    for iteration in 0..10_000 {
        alias
            .set_number(0_i32, JsValue::Number(f64::from(iteration)))
            .unwrap();
    }
    let allocations = TRACKED_ALLOCATIONS.with(|count| count.replace(None).unwrap());
    assert_eq!(allocations, 0);
    assert!(matches!(original.get(0), Some(JsValue::Number(9999.0))));
    assert!(alias.set_number(2_i32, JsValue::Null).is_err());
    alias.set_number(-1_i32, JsValue::Number(3.0)).unwrap();
    assert!(matches!(
        original.get_number(-1_i32),
        Some(JsValue::Number(3.0))
    ));
    let strings = JsArray::from_dense(vec![String::from("unchanged")]);
    let typed = js_value_from_array(&strings, self::strings);
    assert!(typed
        .as_array()
        .unwrap()
        .set_number(0_i32, JsValue::Number(7.0))
        .is_err());
    assert_eq!(strings.get(0).as_deref(), Some("unchanged"));
}

struct OwnedPayload {
    text: String,
    drops: Rc<Cell<usize>>,
}

impl Drop for OwnedPayload {
    fn drop(&mut self) {
        self.drops.set(self.drops.get() + 1);
    }
}

fn payloads(
    array: &JsArrayValue,
    index: JsArrayIndex<'_>,
    visitor: &mut dyn FnMut(JsArrayElement<'_>),
) {
    array.with_native_element::<OwnedPayload, _>(index, |value| {
        if let Some(value) = value {
            visitor(JsArrayElement::String(&value.text));
        }
    });
}

#[test]
fn non_clone_payload_is_destroyed_only_after_the_last_actual_owner() {
    let drops = Rc::new(Cell::new(0));
    let original = JsArray::from_dense(vec![OwnedPayload {
        text: String::from("retained"),
        drops: drops.clone(),
    }]);
    TRACKED_ALLOCATIONS.with(|count| count.set(Some(0)));
    let erased = js_value_from_array(&original, payloads);
    let restored = erased.cast_array::<OwnedPayload>().unwrap();
    let allocations = TRACKED_ALLOCATIONS.with(|count| count.replace(None).unwrap());
    assert_eq!(allocations, 0);
    assert!(restored.ptr_eq(&original));
    drop(original);
    assert_eq!(closed_value_string(&erased).unwrap(), "retained");
    drop(erased);
    assert_eq!(drops.get(), 0);
    drop(restored);
    assert_eq!(drops.get(), 1);
}

#[test]
fn borrowed_reads_do_not_copy_string_buffers_or_allocate_projection_storage() {
    let original = JsArray::from_dense(vec!["native 🧠".repeat(1000)]);
    let erased = js_value_from_array(&original, strings);
    original.with_values(|values| {
        TRACKED_ALLOCATIONS.with(|count| count.set(Some(0)));
        for _iteration in 0..1000 {
            erased.as_array().unwrap().visit_element(0, &mut |element| {
                let JsArrayElement::String(value) = element else {
                    panic!("borrowed native string")
                };
                assert_eq!(value.as_ptr(), values[0].as_ptr());
                assert_eq!(value.len(), values[0].len());
            });
        }
        let allocations = TRACKED_ALLOCATIONS.with(|count| count.replace(None).unwrap());
        assert_eq!(allocations, 0);
    });
}

#[test]
fn projected_native_integer_and_named_property_reads_preserve_exact_values() {
    fn integers(
        array: &JsArrayValue,
        index: JsArrayIndex<'_>,
        visitor: &mut dyn FnMut(JsArrayElement<'_>),
    ) {
        if let Some(value) = array.get_native_element::<u64>(index) {
            visitor(JsArrayElement::Value(&JsValue::UnsignedInteger(value)));
        }
    }
    let values = JsArray::from_dense(vec![9_007_199_254_740_993_u64, u64::MAX]);
    values.set_number(-1.0, u64::MAX);
    let erased = js_value_from_array(&values, integers);
    assert_eq!(
        erased.as_array().unwrap().get_number(-1.0),
        Some(JsValue::UnsignedInteger(u64::MAX))
    );
    assert_eq!(
        closed_value_string(&erased).unwrap(),
        "9007199254740993,18446744073709551615"
    );
    assert_eq!(
        json::stringify(&erased).unwrap().unwrap(),
        "[9007199254740993,18446744073709551615]"
    );
}

#[test]
fn broad_values_recover_only_their_real_storage_and_iteration_retains_live_length() {
    let original = JsArray::from_dense(vec![JsValue::from(String::from("first"))]);
    let erased = JsValue::array(original.clone());
    assert!(erased.cast_array::<String>().is_err());
    let restored = erased.cast_array::<JsValue>().unwrap();
    assert!(restored.ptr_eq(&original));
    let mut iterator = erased.as_array().unwrap().iter_values();
    assert_eq!(iterator.next(), Some(JsValue::from(String::from("first"))));
    original.push(JsValue::UnsignedInteger(u64::MAX));
    drop(erased);
    assert_eq!(iterator.next(), Some(JsValue::UnsignedInteger(u64::MAX)));
    assert_eq!(iterator.next(), None);
}
