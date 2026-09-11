use crate::number::JsNumberValue;
use tsonic_rust_runtime::BigInt;

pub trait IntlNumberInput {
    fn into_intl_decimal(self) -> (String, bool);
}

impl<Value: JsNumberValue> IntlNumberInput for Value {
    fn into_intl_decimal(self) -> (String, bool) {
        let text = self.to_js_decimal_string();
        let negative_zero = text == "0" && self.to_js_f64().is_sign_negative();
        (text, negative_zero)
    }
}

impl IntlNumberInput for &BigInt {
    fn into_intl_decimal(self) -> (String, bool) {
        (self.to_string(), false)
    }
}
