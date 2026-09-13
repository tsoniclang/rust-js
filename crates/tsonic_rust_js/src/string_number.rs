use tsonic_rust_runtime::{Null, Undefined};

#[derive(Clone, Debug, PartialEq)]
pub enum JsStringNumber {
    String(String),
    Number(f64),
    Null,
    Undefined,
}

impl JsStringNumber {
    pub fn from_string(value: String) -> Self {
        Self::String(value)
    }
    pub fn from_number(value: f64) -> Self {
        Self::Number(value)
    }
    pub fn from_int32(value: i32) -> Self {
        Self::Number(f64::from(value))
    }
    pub fn from_null(_value: Null) -> Self {
        Self::Null
    }
    pub fn from_undefined(_value: Undefined) -> Self {
        Self::Undefined
    }

    pub fn type_of(&self) -> String {
        match self {
            Self::String(_) => "string",
            Self::Number(_) => "number",
            Self::Null => "object",
            Self::Undefined => "undefined",
        }
        .to_owned()
    }

    pub fn as_string(&self) -> String {
        match self {
            Self::String(value) => value.clone(),
            _ => unreachable!("checked union refinement must select string"),
        }
    }

    pub fn as_number(&self) -> f64 {
        match self {
            Self::Number(value) => *value,
            _ => unreachable!("checked union refinement must select number"),
        }
    }

    pub fn as_null(&self) -> Null {
        match self {
            Self::Null => Null,
            _ => unreachable!("checked union refinement must select null"),
        }
    }

    pub fn as_undefined(&self) -> Undefined {
        match self {
            Self::Undefined => Undefined,
            _ => unreachable!("checked union refinement must select undefined"),
        }
    }
}

macro_rules! scalar_equality {
    ($type:ty, $variant:ident) => {
        impl PartialEq<$type> for JsStringNumber {
            fn eq(&self, other: &$type) -> bool {
                matches!(self, Self::$variant(value) if value == other)
            }
        }
        impl PartialEq<JsStringNumber> for $type {
            fn eq(&self, other: &JsStringNumber) -> bool { other == self }
        }
    };
}
scalar_equality!(String, String);
scalar_equality!(f64, Number);

impl PartialEq<&str> for JsStringNumber {
    fn eq(&self, other: &&str) -> bool {
        matches!(self, Self::String(value) if value == other)
    }
}
impl PartialEq<JsStringNumber> for &str {
    fn eq(&self, other: &JsStringNumber) -> bool {
        other == self
    }
}
impl PartialEq<Null> for JsStringNumber {
    fn eq(&self, _other: &Null) -> bool {
        matches!(self, Self::Null)
    }
}
impl PartialEq<JsStringNumber> for Null {
    fn eq(&self, other: &JsStringNumber) -> bool {
        other == self
    }
}
impl PartialEq<Undefined> for JsStringNumber {
    fn eq(&self, _other: &Undefined) -> bool {
        matches!(self, Self::Undefined)
    }
}
impl PartialEq<JsStringNumber> for Undefined {
    fn eq(&self, other: &JsStringNumber) -> bool {
        other == self
    }
}
