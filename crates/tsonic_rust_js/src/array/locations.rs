use super::js_array::{canonical_array_index, JsArray};
use tsonic_rust_runtime::location::LocationSegment;
use tsonic_rust_runtime::Location;

impl<T: Clone + 'static> JsArray<T> {
    pub fn element_location(&self, index: f64) -> Location<T> {
        let segment = canonical_array_index(index).map_or_else(
            || LocationSegment::Member(crate::number::to_string(index)),
            LocationSegment::Index,
        );
        let read = self.clone();
        let write = self.clone();
        Location::bind_projected(
            self.clone(),
            segment,
            move || match read.get_number(index) {
                Some(value) => value,
                None => panic!("typed array location read requires a present element"),
            },
            move |value| write.set_number(index, value),
        )
    }
}
