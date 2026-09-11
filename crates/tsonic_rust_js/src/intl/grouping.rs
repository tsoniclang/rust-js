#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IntlGrouping {
    Disabled,
    Strategy(String),
}

impl IntlGrouping {
    pub fn type_of(&self) -> String {
        match self {
            Self::Disabled => "boolean",
            Self::Strategy(_) => "string",
        }
        .to_owned()
    }

    pub fn as_string(&self) -> String {
        match self {
            Self::Strategy(value) => value.clone(),
            Self::Disabled => unreachable!("checked flow selected a string grouping strategy"),
        }
    }

    pub fn as_bool(&self) -> bool {
        match self {
            Self::Disabled => false,
            Self::Strategy(_) => unreachable!("checked flow selected disabled grouping"),
        }
    }
}

impl PartialEq<String> for IntlGrouping {
    fn eq(&self, other: &String) -> bool {
        matches!(self, Self::Strategy(value) if value == other)
    }
}

impl PartialEq<IntlGrouping> for String {
    fn eq(&self, other: &IntlGrouping) -> bool {
        other == self
    }
}

impl PartialEq<&str> for IntlGrouping {
    fn eq(&self, other: &&str) -> bool {
        matches!(self, Self::Strategy(value) if value == other)
    }
}

impl PartialEq<IntlGrouping> for &str {
    fn eq(&self, other: &IntlGrouping) -> bool {
        other == self
    }
}

impl PartialEq<bool> for IntlGrouping {
    fn eq(&self, other: &bool) -> bool {
        !other && matches!(self, Self::Disabled)
    }
}

impl PartialEq<IntlGrouping> for bool {
    fn eq(&self, other: &IntlGrouping) -> bool {
        other == self
    }
}
