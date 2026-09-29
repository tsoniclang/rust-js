use crate::array::JsArray;
use tsonic_rust_runtime::Record;

pub fn keys<Value>(record: &Record<String, Value>) -> JsArray<String> {
    JsArray::from_dense(record.keys())
}

pub fn values<Value: Clone>(record: &Record<String, Value>) -> JsArray<Value> {
    JsArray::from_dense(record.values())
}

pub fn entries<Value: Clone>(record: &Record<String, Value>) -> JsArray<(String, Value)> {
    JsArray::from_dense(record.entries())
}

pub fn assign<Value: Clone>(
    target: &Record<String, Value>,
    source: &Record<String, Value>,
) -> Record<String, Value> {
    target.extend(source);
    target.clone()
}
