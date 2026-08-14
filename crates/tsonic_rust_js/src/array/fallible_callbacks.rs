use super::js_array::JsArray;
use tsonic_rust_runtime::{JsError, JsErrorKind};

impl<T> JsArray<T> {
    pub fn try_sort_zero<E, F>(&self, mut compare: F) -> Result<Self, E>
    where
        T: Clone,
        F: FnMut() -> Result<f64, E>,
    {
        self.try_sort_present_by(|_, _| compare())
    }

    pub fn try_sort_value<E, F>(&self, mut compare: F) -> Result<Self, E>
    where
        T: Clone,
        F: FnMut(T) -> Result<f64, E>,
    {
        self.try_sort_present_by(|left, _| compare(left))
    }

    pub fn try_sort<E, F>(&self, compare: F) -> Result<Self, E>
    where
        T: Clone,
        F: FnMut(T, T) -> Result<f64, E>,
    {
        self.try_sort_present_by(compare)
    }

    fn try_map_with<U, E, F>(&self, mut mapper: F) -> Result<JsArray<U>, E>
    where
        T: Clone,
        F: FnMut(T, f64, Self) -> Result<U, E>,
    {
        let length = self.len();
        let output = JsArray::with_length(length);
        for index in 0..length {
            if let Some(value) = self.get(index) {
                output.set(index, mapper(value, index as f64, self.clone())?);
            }
        }
        Ok(output)
    }

    pub fn try_map_zero<U, E, F>(&self, mut mapper: F) -> Result<JsArray<U>, E>
    where
        T: Clone,
        F: FnMut() -> Result<U, E>,
    {
        self.try_map_with(|_, _, _| mapper())
    }

    pub fn try_map<U, E, F>(&self, mut mapper: F) -> Result<JsArray<U>, E>
    where
        T: Clone,
        F: FnMut(T) -> Result<U, E>,
    {
        self.try_map_with(|value, _, _| mapper(value))
    }

    pub fn try_map_with_index<U, E, F>(&self, mut mapper: F) -> Result<JsArray<U>, E>
    where
        T: Clone,
        F: FnMut(T, f64) -> Result<U, E>,
    {
        self.try_map_with(|value, index, _| mapper(value, index))
    }

    pub fn try_map_with_array<U, E, F>(&self, mapper: F) -> Result<JsArray<U>, E>
    where
        T: Clone,
        F: FnMut(T, f64, Self) -> Result<U, E>,
    {
        self.try_map_with(mapper)
    }

    fn try_filter_with<E, F>(&self, mut predicate: F) -> Result<Self, E>
    where
        T: Clone,
        F: FnMut(T, f64, Self) -> Result<bool, E>,
    {
        let length = self.len();
        let output = Self::new();
        for index in 0..length {
            if let Some(value) = self.get(index) {
                if predicate(value.clone(), index as f64, self.clone())? {
                    output.push(value);
                }
            }
        }
        Ok(output)
    }

    pub fn try_filter_zero<E, F>(&self, mut predicate: F) -> Result<Self, E>
    where
        T: Clone,
        F: FnMut() -> Result<bool, E>,
    {
        self.try_filter_with(|_, _, _| predicate())
    }

    pub fn try_filter<E, F>(&self, mut predicate: F) -> Result<Self, E>
    where
        T: Clone,
        F: FnMut(T) -> Result<bool, E>,
    {
        self.try_filter_with(|value, _, _| predicate(value))
    }

    pub fn try_filter_with_index<E, F>(&self, mut predicate: F) -> Result<Self, E>
    where
        T: Clone,
        F: FnMut(T, f64) -> Result<bool, E>,
    {
        self.try_filter_with(|value, index, _| predicate(value, index))
    }

    pub fn try_filter_with_array<E, F>(&self, predicate: F) -> Result<Self, E>
    where
        T: Clone,
        F: FnMut(T, f64, Self) -> Result<bool, E>,
    {
        self.try_filter_with(predicate)
    }

    fn try_reduce_with<U, E, F>(&self, initial: U, mut reducer: F) -> Result<U, E>
    where
        T: Clone,
        F: FnMut(U, T, f64, Self) -> Result<U, E>,
    {
        let length = self.len();
        let mut accumulator = initial;
        for index in 0..length {
            if let Some(value) = self.get(index) {
                accumulator = reducer(accumulator, value, index as f64, self.clone())?;
            }
        }
        Ok(accumulator)
    }

    pub fn try_reduce_zero<U, E, F>(&self, initial: U, mut reducer: F) -> Result<U, E>
    where
        T: Clone,
        F: FnMut() -> Result<U, E>,
    {
        self.try_reduce_with(initial, |_, _, _, _| reducer())
    }

    pub fn try_reduce_accumulator<U, E, F>(&self, initial: U, mut reducer: F) -> Result<U, E>
    where
        T: Clone,
        F: FnMut(U) -> Result<U, E>,
    {
        self.try_reduce_with(initial, |accumulator, _, _, _| reducer(accumulator))
    }

    pub fn try_reduce<U, E, F>(&self, initial: U, mut reducer: F) -> Result<U, E>
    where
        T: Clone,
        F: FnMut(U, T) -> Result<U, E>,
    {
        self.try_reduce_with(initial, |accumulator, value, _, _| {
            reducer(accumulator, value)
        })
    }

    pub fn try_reduce_with_index<U, E, F>(&self, initial: U, mut reducer: F) -> Result<U, E>
    where
        T: Clone,
        F: FnMut(U, T, f64) -> Result<U, E>,
    {
        self.try_reduce_with(initial, |accumulator, value, index, _| {
            reducer(accumulator, value, index)
        })
    }

    pub fn try_reduce_with_array<U, E, F>(&self, initial: U, reducer: F) -> Result<U, E>
    where
        T: Clone,
        F: FnMut(U, T, f64, Self) -> Result<U, E>,
    {
        self.try_reduce_with(initial, reducer)
    }

    fn try_reduce_from_first_with<E, F>(&self, mut reducer: F) -> Result<T, E>
    where
        T: Clone,
        E: From<JsError>,
        F: FnMut(T, T, f64, Self) -> Result<T, E>,
    {
        let length = self.len();
        let Some((first_index, mut accumulator)) =
            (0..length).find_map(|index| self.get(index).map(|value| (index, value)))
        else {
            return Err(JsError::new(
                JsErrorKind::TypeError,
                "Reduce of empty array with no initial value",
            )
            .into());
        };
        for index in first_index + 1..length {
            if let Some(value) = self.get(index) {
                accumulator = reducer(accumulator, value, index as f64, self.clone())?;
            }
        }
        Ok(accumulator)
    }

    pub fn try_reduce_from_first_zero<E, F>(&self, mut reducer: F) -> Result<T, E>
    where
        T: Clone,
        E: From<JsError>,
        F: FnMut() -> Result<T, E>,
    {
        self.try_reduce_from_first_with(|_, _, _, _| reducer())
    }

    pub fn try_reduce_from_first_accumulator<E, F>(&self, mut reducer: F) -> Result<T, E>
    where
        T: Clone,
        E: From<JsError>,
        F: FnMut(T) -> Result<T, E>,
    {
        self.try_reduce_from_first_with(|accumulator, _, _, _| reducer(accumulator))
    }

    pub fn try_reduce_from_first<E, F>(&self, mut reducer: F) -> Result<T, E>
    where
        T: Clone,
        E: From<JsError>,
        F: FnMut(T, T) -> Result<T, E>,
    {
        self.try_reduce_from_first_with(|accumulator, value, _, _| reducer(accumulator, value))
    }

    pub fn try_reduce_from_first_with_index<E, F>(&self, mut reducer: F) -> Result<T, E>
    where
        T: Clone,
        E: From<JsError>,
        F: FnMut(T, T, f64) -> Result<T, E>,
    {
        self.try_reduce_from_first_with(|accumulator, value, index, _| {
            reducer(accumulator, value, index)
        })
    }

    pub fn try_reduce_from_first_with_array<E, F>(&self, reducer: F) -> Result<T, E>
    where
        T: Clone,
        E: From<JsError>,
        F: FnMut(T, T, f64, Self) -> Result<T, E>,
    {
        self.try_reduce_from_first_with(reducer)
    }

    fn try_for_each_with<E, F>(&self, mut callback: F) -> Result<(), E>
    where
        T: Clone,
        F: FnMut(T, f64, Self) -> Result<(), E>,
    {
        let length = self.len();
        for index in 0..length {
            if let Some(value) = self.get(index) {
                callback(value, index as f64, self.clone())?;
            }
        }
        Ok(())
    }

    pub fn try_for_each_zero<E, F>(&self, mut callback: F) -> Result<(), E>
    where
        T: Clone,
        F: FnMut() -> Result<(), E>,
    {
        self.try_for_each_with(|_, _, _| callback())
    }

    pub fn try_for_each_value<E, F>(&self, mut callback: F) -> Result<(), E>
    where
        T: Clone,
        F: FnMut(T) -> Result<(), E>,
    {
        self.try_for_each_with(|value, _, _| callback(value))
    }

    pub fn try_for_each_value_index<E, F>(&self, mut callback: F) -> Result<(), E>
    where
        T: Clone,
        F: FnMut(T, f64) -> Result<(), E>,
    {
        self.try_for_each_with(|value, index, _| callback(value, index))
    }

    pub fn try_for_each<E, F>(&self, callback: F) -> Result<(), E>
    where
        T: Clone,
        F: FnMut(T, f64, Self) -> Result<(), E>,
    {
        self.try_for_each_with(callback)
    }

    fn try_find_match_with<E, F>(
        &self,
        reverse: bool,
        mut predicate: F,
    ) -> Result<Option<(usize, T)>, E>
    where
        T: Clone,
        F: FnMut(T, f64, Self) -> Result<bool, E>,
    {
        let length = self.len();
        for offset in 0..length {
            let index = if reverse { length - offset - 1 } else { offset };
            if let Some(value) = self.get(index) {
                if predicate(value.clone(), index as f64, self.clone())? {
                    return Ok(Some((index, value)));
                }
            }
        }
        Ok(None)
    }

    pub fn try_find_zero<E, F>(&self, mut predicate: F) -> Result<Option<T>, E>
    where
        T: Clone,
        F: FnMut() -> Result<bool, E>,
    {
        Ok(self
            .try_find_match_with(false, |_, _, _| predicate())?
            .map(|(_, value)| value))
    }

    pub fn try_find<E, F>(&self, mut predicate: F) -> Result<Option<T>, E>
    where
        T: Clone,
        F: FnMut(T) -> Result<bool, E>,
    {
        Ok(self
            .try_find_match_with(false, |value, _, _| predicate(value))?
            .map(|(_, value)| value))
    }

    pub fn try_find_with_index<E, F>(&self, mut predicate: F) -> Result<Option<T>, E>
    where
        T: Clone,
        F: FnMut(T, f64) -> Result<bool, E>,
    {
        Ok(self
            .try_find_match_with(false, |value, index, _| predicate(value, index))?
            .map(|(_, value)| value))
    }

    pub fn try_find_with_array<E, F>(&self, predicate: F) -> Result<Option<T>, E>
    where
        T: Clone,
        F: FnMut(T, f64, Self) -> Result<bool, E>,
    {
        Ok(self
            .try_find_match_with(false, predicate)?
            .map(|(_, value)| value))
    }

    pub fn try_find_index_zero<E, F>(&self, mut predicate: F) -> Result<isize, E>
    where
        T: Clone,
        F: FnMut() -> Result<bool, E>,
    {
        Ok(self
            .try_find_match_with(false, |_, _, _| predicate())?
            .map_or(-1, |(index, _)| index as isize))
    }

    pub fn try_find_index<E, F>(&self, mut predicate: F) -> Result<isize, E>
    where
        T: Clone,
        F: FnMut(T) -> Result<bool, E>,
    {
        Ok(self
            .try_find_match_with(false, |value, _, _| predicate(value))?
            .map_or(-1, |(index, _)| index as isize))
    }

    pub fn try_find_index_with_index<E, F>(&self, mut predicate: F) -> Result<isize, E>
    where
        T: Clone,
        F: FnMut(T, f64) -> Result<bool, E>,
    {
        Ok(self
            .try_find_match_with(false, |value, index, _| predicate(value, index))?
            .map_or(-1, |(index, _)| index as isize))
    }

    pub fn try_find_index_with_array<E, F>(&self, predicate: F) -> Result<isize, E>
    where
        T: Clone,
        F: FnMut(T, f64, Self) -> Result<bool, E>,
    {
        Ok(self
            .try_find_match_with(false, predicate)?
            .map_or(-1, |(index, _)| index as isize))
    }

    pub fn try_find_last_zero<E, F>(&self, mut predicate: F) -> Result<Option<T>, E>
    where
        T: Clone,
        F: FnMut() -> Result<bool, E>,
    {
        Ok(self
            .try_find_match_with(true, |_, _, _| predicate())?
            .map(|(_, value)| value))
    }

    pub fn try_find_last<E, F>(&self, mut predicate: F) -> Result<Option<T>, E>
    where
        T: Clone,
        F: FnMut(T) -> Result<bool, E>,
    {
        Ok(self
            .try_find_match_with(true, |value, _, _| predicate(value))?
            .map(|(_, value)| value))
    }

    pub fn try_find_last_with_index<E, F>(&self, mut predicate: F) -> Result<Option<T>, E>
    where
        T: Clone,
        F: FnMut(T, f64) -> Result<bool, E>,
    {
        Ok(self
            .try_find_match_with(true, |value, index, _| predicate(value, index))?
            .map(|(_, value)| value))
    }

    pub fn try_find_last_with_array<E, F>(&self, predicate: F) -> Result<Option<T>, E>
    where
        T: Clone,
        F: FnMut(T, f64, Self) -> Result<bool, E>,
    {
        Ok(self
            .try_find_match_with(true, predicate)?
            .map(|(_, value)| value))
    }

    pub fn try_find_last_index_zero<E, F>(&self, mut predicate: F) -> Result<isize, E>
    where
        T: Clone,
        F: FnMut() -> Result<bool, E>,
    {
        Ok(self
            .try_find_match_with(true, |_, _, _| predicate())?
            .map_or(-1, |(index, _)| index as isize))
    }

    pub fn try_find_last_index<E, F>(&self, mut predicate: F) -> Result<isize, E>
    where
        T: Clone,
        F: FnMut(T) -> Result<bool, E>,
    {
        Ok(self
            .try_find_match_with(true, |value, _, _| predicate(value))?
            .map_or(-1, |(index, _)| index as isize))
    }

    pub fn try_find_last_index_with_index<E, F>(&self, mut predicate: F) -> Result<isize, E>
    where
        T: Clone,
        F: FnMut(T, f64) -> Result<bool, E>,
    {
        Ok(self
            .try_find_match_with(true, |value, index, _| predicate(value, index))?
            .map_or(-1, |(index, _)| index as isize))
    }

    pub fn try_find_last_index_with_array<E, F>(&self, predicate: F) -> Result<isize, E>
    where
        T: Clone,
        F: FnMut(T, f64, Self) -> Result<bool, E>,
    {
        Ok(self
            .try_find_match_with(true, predicate)?
            .map_or(-1, |(index, _)| index as isize))
    }

    fn try_quantify_with<E, F>(&self, every: bool, mut predicate: F) -> Result<bool, E>
    where
        T: Clone,
        F: FnMut(T, f64, Self) -> Result<bool, E>,
    {
        let length = self.len();
        for index in 0..length {
            if let Some(value) = self.get(index) {
                let matched = predicate(value, index as f64, self.clone())?;
                if matched != every {
                    return Ok(!every);
                }
            }
        }
        Ok(every)
    }

    pub fn try_some_zero<E, F>(&self, mut predicate: F) -> Result<bool, E>
    where
        T: Clone,
        F: FnMut() -> Result<bool, E>,
    {
        self.try_quantify_with(false, |_, _, _| predicate())
    }

    pub fn try_some<E, F>(&self, mut predicate: F) -> Result<bool, E>
    where
        T: Clone,
        F: FnMut(T) -> Result<bool, E>,
    {
        self.try_quantify_with(false, |value, _, _| predicate(value))
    }

    pub fn try_some_with_index<E, F>(&self, mut predicate: F) -> Result<bool, E>
    where
        T: Clone,
        F: FnMut(T, f64) -> Result<bool, E>,
    {
        self.try_quantify_with(false, |value, index, _| predicate(value, index))
    }

    pub fn try_some_with_array<E, F>(&self, predicate: F) -> Result<bool, E>
    where
        T: Clone,
        F: FnMut(T, f64, Self) -> Result<bool, E>,
    {
        self.try_quantify_with(false, predicate)
    }

    pub fn try_every_zero<E, F>(&self, mut predicate: F) -> Result<bool, E>
    where
        T: Clone,
        F: FnMut() -> Result<bool, E>,
    {
        self.try_quantify_with(true, |_, _, _| predicate())
    }

    pub fn try_every<E, F>(&self, mut predicate: F) -> Result<bool, E>
    where
        T: Clone,
        F: FnMut(T) -> Result<bool, E>,
    {
        self.try_quantify_with(true, |value, _, _| predicate(value))
    }

    pub fn try_every_with_index<E, F>(&self, mut predicate: F) -> Result<bool, E>
    where
        T: Clone,
        F: FnMut(T, f64) -> Result<bool, E>,
    {
        self.try_quantify_with(true, |value, index, _| predicate(value, index))
    }

    pub fn try_every_with_array<E, F>(&self, predicate: F) -> Result<bool, E>
    where
        T: Clone,
        F: FnMut(T, f64, Self) -> Result<bool, E>,
    {
        self.try_quantify_with(true, predicate)
    }
}
