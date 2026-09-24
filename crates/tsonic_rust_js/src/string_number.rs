#[derive(Clone, Debug, PartialEq)]
pub enum JsStringNumber {
    String(String),
    Number(f64),
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

    pub fn type_of(&self) -> String {
        match self {
            Self::String(_) => "string",
            Self::Number(_) => "number",
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
