//! JavaScript Boolean primitive operations.

pub fn to_string(value: bool) -> String {
    if value {
        "true".to_string()
    } else {
        "false".to_string()
    }
}

pub fn value_of(value: bool) -> bool {
    value
}
