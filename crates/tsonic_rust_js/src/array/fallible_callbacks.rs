use super::js_array::JsArray;
use tsonic_rust_runtime::{JsError, JsErrorKind, TsonicResult};

impl<T> JsArray<T> {
    pub fn try_sort_zero<F>(&self, mut compare: F) -> TsonicResult<Self>
    where
        T: Clone,
        F: FnMut() -> TsonicResult<f64>,
    {
        self.try_sort_present_by(|_, _| compare())
    }

    pub fn try_sort_value<F>(&self, mut compare: F) -> TsonicResult<Self>
    where
        T: Clone,
        F: FnMut(T) -> TsonicResult<f64>,
    {
        self.try_sort_present_by(|left, _| compare(left))
    }

    pub fn try_sort<F>(&self, compare: F) -> TsonicResult<Self>
    where
        T: Clone,
        F: FnMut(T, T) -> TsonicResult<f64>,
    {
        self.try_sort_present_by(compare)
    }

    fn try_map_with<U, F>(&self, mut mapper: F) -> TsonicResult<JsArray<U>>
    where
        T: Clone,
        F: FnMut(T, f64, Self) -> TsonicResult<U>,
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

    pub fn try_map_zero<U, F>(&self, mut mapper: F) -> TsonicResult<JsArray<U>>
    where
        T: Clone,
        F: FnMut() -> TsonicResult<U>,
    {
        self.try_map_with(|_, _, _| mapper())
    }

    pub fn try_map<U, F>(&self, mut mapper: F) -> TsonicResult<JsArray<U>>
    where
        T: Clone,
        F: FnMut(T) -> TsonicResult<U>,
    {
        self.try_map_with(|value, _, _| mapper(value))
    }

    pub fn try_map_with_index<U, F>(&self, mut mapper: F) -> TsonicResult<JsArray<U>>
    where
        T: Clone,
        F: FnMut(T, f64) -> TsonicResult<U>,
    {
        self.try_map_with(|value, index, _| mapper(value, index))
    }

    pub fn try_map_with_array<U, F>(&self, mapper: F) -> TsonicResult<JsArray<U>>
    where
        T: Clone,
        F: FnMut(T, f64, Self) -> TsonicResult<U>,
    {
        self.try_map_with(mapper)
    }

    fn try_filter_with<F>(&self, mut predicate: F) -> TsonicResult<Self>
    where
        T: Clone,
        F: FnMut(T, f64, Self) -> TsonicResult<bool>,
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

    pub fn try_filter_zero<F>(&self, mut predicate: F) -> TsonicResult<Self>
    where
        T: Clone,
        F: FnMut() -> TsonicResult<bool>,
    {
        self.try_filter_with(|_, _, _| predicate())
    }

    pub fn try_filter<F>(&self, mut predicate: F) -> TsonicResult<Self>
    where
        T: Clone,
        F: FnMut(T) -> TsonicResult<bool>,
    {
        self.try_filter_with(|value, _, _| predicate(value))
    }

    pub fn try_filter_with_index<F>(&self, mut predicate: F) -> TsonicResult<Self>
    where
        T: Clone,
        F: FnMut(T, f64) -> TsonicResult<bool>,
    {
        self.try_filter_with(|value, index, _| predicate(value, index))
    }

    pub fn try_filter_with_array<F>(&self, predicate: F) -> TsonicResult<Self>
    where
        T: Clone,
        F: FnMut(T, f64, Self) -> TsonicResult<bool>,
    {
        self.try_filter_with(predicate)
    }

    fn try_reduce_with<U, F>(&self, initial: U, mut reducer: F) -> TsonicResult<U>
    where
        T: Clone,
        F: FnMut(U, T, f64, Self) -> TsonicResult<U>,
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

    pub fn try_reduce_zero<U, F>(&self, initial: U, mut reducer: F) -> TsonicResult<U>
    where
        T: Clone,
        F: FnMut() -> TsonicResult<U>,
    {
        self.try_reduce_with(initial, |_, _, _, _| reducer())
    }

    pub fn try_reduce_accumulator<U, F>(&self, initial: U, mut reducer: F) -> TsonicResult<U>
    where
        T: Clone,
        F: FnMut(U) -> TsonicResult<U>,
    {
        self.try_reduce_with(initial, |accumulator, _, _, _| reducer(accumulator))
    }

    pub fn try_reduce<U, F>(&self, initial: U, mut reducer: F) -> TsonicResult<U>
    where
        T: Clone,
        F: FnMut(U, T) -> TsonicResult<U>,
    {
        self.try_reduce_with(initial, |accumulator, value, _, _| {
            reducer(accumulator, value)
        })
    }

    pub fn try_reduce_with_index<U, F>(&self, initial: U, mut reducer: F) -> TsonicResult<U>
    where
        T: Clone,
        F: FnMut(U, T, f64) -> TsonicResult<U>,
    {
        self.try_reduce_with(initial, |accumulator, value, index, _| {
            reducer(accumulator, value, index)
        })
    }

    pub fn try_reduce_with_array<U, F>(&self, initial: U, reducer: F) -> TsonicResult<U>
    where
        T: Clone,
        F: FnMut(U, T, f64, Self) -> TsonicResult<U>,
    {
        self.try_reduce_with(initial, reducer)
    }

    fn try_reduce_from_first_with<F>(&self, mut reducer: F) -> TsonicResult<T>
    where
        T: Clone,
        F: FnMut(T, T, f64, Self) -> TsonicResult<T>,
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

    pub fn try_reduce_from_first_zero<F>(&self, mut reducer: F) -> TsonicResult<T>
    where
        T: Clone,
        F: FnMut() -> TsonicResult<T>,
    {
        self.try_reduce_from_first_with(|_, _, _, _| reducer())
    }

    pub fn try_reduce_from_first_accumulator<F>(&self, mut reducer: F) -> TsonicResult<T>
    where
        T: Clone,
        F: FnMut(T) -> TsonicResult<T>,
    {
        self.try_reduce_from_first_with(|accumulator, _, _, _| reducer(accumulator))
    }

    pub fn try_reduce_from_first<F>(&self, mut reducer: F) -> TsonicResult<T>
    where
        T: Clone,
        F: FnMut(T, T) -> TsonicResult<T>,
    {
        self.try_reduce_from_first_with(|accumulator, value, _, _| reducer(accumulator, value))
    }

    pub fn try_reduce_from_first_with_index<F>(&self, mut reducer: F) -> TsonicResult<T>
    where
        T: Clone,
        F: FnMut(T, T, f64) -> TsonicResult<T>,
    {
        self.try_reduce_from_first_with(|accumulator, value, index, _| {
            reducer(accumulator, value, index)
        })
    }

    pub fn try_reduce_from_first_with_array<F>(&self, reducer: F) -> TsonicResult<T>
    where
        T: Clone,
        F: FnMut(T, T, f64, Self) -> TsonicResult<T>,
    {
        self.try_reduce_from_first_with(reducer)
    }

    fn try_for_each_with<F>(&self, mut callback: F) -> TsonicResult<()>
    where
        T: Clone,
        F: FnMut(T, f64, Self) -> TsonicResult<()>,
    {
        let length = self.len();
        for index in 0..length {
            if let Some(value) = self.get(index) {
                callback(value, index as f64, self.clone())?;
            }
        }
        Ok(())
    }

    pub fn try_for_each_zero<F>(&self, mut callback: F) -> TsonicResult<()>
    where
        T: Clone,
        F: FnMut() -> TsonicResult<()>,
    {
        self.try_for_each_with(|_, _, _| callback())
    }

    pub fn try_for_each_value<F>(&self, mut callback: F) -> TsonicResult<()>
    where
        T: Clone,
        F: FnMut(T) -> TsonicResult<()>,
    {
        self.try_for_each_with(|value, _, _| callback(value))
    }

    pub fn try_for_each_value_index<F>(&self, mut callback: F) -> TsonicResult<()>
    where
        T: Clone,
        F: FnMut(T, f64) -> TsonicResult<()>,
    {
        self.try_for_each_with(|value, index, _| callback(value, index))
    }

    pub fn try_for_each<F>(&self, callback: F) -> TsonicResult<()>
    where
        T: Clone,
        F: FnMut(T, f64, Self) -> TsonicResult<()>,
    {
        self.try_for_each_with(callback)
    }

    fn try_find_match_with<F>(
        &self,
        reverse: bool,
        mut predicate: F,
    ) -> TsonicResult<Option<(usize, T)>>
    where
        T: Clone,
        F: FnMut(T, f64, Self) -> TsonicResult<bool>,
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

    pub fn try_find_zero<F>(&self, mut predicate: F) -> TsonicResult<Option<T>>
    where
        T: Clone,
        F: FnMut() -> TsonicResult<bool>,
    {
        Ok(self
            .try_find_match_with(false, |_, _, _| predicate())?
            .map(|(_, value)| value))
    }

    pub fn try_find<F>(&self, mut predicate: F) -> TsonicResult<Option<T>>
    where
        T: Clone,
        F: FnMut(T) -> TsonicResult<bool>,
    {
        Ok(self
            .try_find_match_with(false, |value, _, _| predicate(value))?
            .map(|(_, value)| value))
    }

    pub fn try_find_with_index<F>(&self, mut predicate: F) -> TsonicResult<Option<T>>
    where
        T: Clone,
        F: FnMut(T, f64) -> TsonicResult<bool>,
    {
        Ok(self
            .try_find_match_with(false, |value, index, _| predicate(value, index))?
            .map(|(_, value)| value))
    }

    pub fn try_find_with_array<F>(&self, predicate: F) -> TsonicResult<Option<T>>
    where
        T: Clone,
        F: FnMut(T, f64, Self) -> TsonicResult<bool>,
    {
        Ok(self
            .try_find_match_with(false, predicate)?
            .map(|(_, value)| value))
    }

    pub fn try_find_index_zero<F>(&self, mut predicate: F) -> TsonicResult<isize>
    where
        T: Clone,
        F: FnMut() -> TsonicResult<bool>,
    {
        Ok(self
            .try_find_match_with(false, |_, _, _| predicate())?
            .map_or(-1, |(index, _)| index as isize))
    }

    pub fn try_find_index<F>(&self, mut predicate: F) -> TsonicResult<isize>
    where
        T: Clone,
        F: FnMut(T) -> TsonicResult<bool>,
    {
        Ok(self
            .try_find_match_with(false, |value, _, _| predicate(value))?
            .map_or(-1, |(index, _)| index as isize))
    }

    pub fn try_find_index_with_index<F>(&self, mut predicate: F) -> TsonicResult<isize>
    where
        T: Clone,
        F: FnMut(T, f64) -> TsonicResult<bool>,
    {
        Ok(self
            .try_find_match_with(false, |value, index, _| predicate(value, index))?
            .map_or(-1, |(index, _)| index as isize))
    }

    pub fn try_find_index_with_array<F>(&self, predicate: F) -> TsonicResult<isize>
    where
        T: Clone,
        F: FnMut(T, f64, Self) -> TsonicResult<bool>,
    {
        Ok(self
            .try_find_match_with(false, predicate)?
            .map_or(-1, |(index, _)| index as isize))
    }

    pub fn try_find_last_zero<F>(&self, mut predicate: F) -> TsonicResult<Option<T>>
    where
        T: Clone,
        F: FnMut() -> TsonicResult<bool>,
    {
        Ok(self
            .try_find_match_with(true, |_, _, _| predicate())?
            .map(|(_, value)| value))
    }

    pub fn try_find_last<F>(&self, mut predicate: F) -> TsonicResult<Option<T>>
    where
        T: Clone,
        F: FnMut(T) -> TsonicResult<bool>,
    {
        Ok(self
            .try_find_match_with(true, |value, _, _| predicate(value))?
            .map(|(_, value)| value))
    }

    pub fn try_find_last_with_index<F>(&self, mut predicate: F) -> TsonicResult<Option<T>>
    where
        T: Clone,
        F: FnMut(T, f64) -> TsonicResult<bool>,
    {
        Ok(self
            .try_find_match_with(true, |value, index, _| predicate(value, index))?
            .map(|(_, value)| value))
    }

    pub fn try_find_last_with_array<F>(&self, predicate: F) -> TsonicResult<Option<T>>
    where
        T: Clone,
        F: FnMut(T, f64, Self) -> TsonicResult<bool>,
    {
        Ok(self
            .try_find_match_with(true, predicate)?
            .map(|(_, value)| value))
    }

    pub fn try_find_last_index_zero<F>(&self, mut predicate: F) -> TsonicResult<isize>
    where
        T: Clone,
        F: FnMut() -> TsonicResult<bool>,
    {
        Ok(self
            .try_find_match_with(true, |_, _, _| predicate())?
            .map_or(-1, |(index, _)| index as isize))
    }

    pub fn try_find_last_index<F>(&self, mut predicate: F) -> TsonicResult<isize>
    where
        T: Clone,
        F: FnMut(T) -> TsonicResult<bool>,
    {
        Ok(self
            .try_find_match_with(true, |value, _, _| predicate(value))?
            .map_or(-1, |(index, _)| index as isize))
    }

    pub fn try_find_last_index_with_index<F>(&self, mut predicate: F) -> TsonicResult<isize>
    where
        T: Clone,
        F: FnMut(T, f64) -> TsonicResult<bool>,
    {
        Ok(self
            .try_find_match_with(true, |value, index, _| predicate(value, index))?
            .map_or(-1, |(index, _)| index as isize))
    }

    pub fn try_find_last_index_with_array<F>(&self, predicate: F) -> TsonicResult<isize>
    where
        T: Clone,
        F: FnMut(T, f64, Self) -> TsonicResult<bool>,
    {
        Ok(self
            .try_find_match_with(true, predicate)?
            .map_or(-1, |(index, _)| index as isize))
    }

    fn try_quantify_with<F>(&self, every: bool, mut predicate: F) -> TsonicResult<bool>
    where
        T: Clone,
        F: FnMut(T, f64, Self) -> TsonicResult<bool>,
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

    pub fn try_some_zero<F>(&self, mut predicate: F) -> TsonicResult<bool>
    where
        T: Clone,
        F: FnMut() -> TsonicResult<bool>,
    {
        self.try_quantify_with(false, |_, _, _| predicate())
    }

    pub fn try_some<F>(&self, mut predicate: F) -> TsonicResult<bool>
    where
        T: Clone,
        F: FnMut(T) -> TsonicResult<bool>,
    {
        self.try_quantify_with(false, |value, _, _| predicate(value))
    }

    pub fn try_some_with_index<F>(&self, mut predicate: F) -> TsonicResult<bool>
    where
        T: Clone,
        F: FnMut(T, f64) -> TsonicResult<bool>,
    {
        self.try_quantify_with(false, |value, index, _| predicate(value, index))
    }

    pub fn try_some_with_array<F>(&self, predicate: F) -> TsonicResult<bool>
    where
        T: Clone,
        F: FnMut(T, f64, Self) -> TsonicResult<bool>,
    {
        self.try_quantify_with(false, predicate)
    }

    pub fn try_every_zero<F>(&self, mut predicate: F) -> TsonicResult<bool>
    where
        T: Clone,
        F: FnMut() -> TsonicResult<bool>,
    {
        self.try_quantify_with(true, |_, _, _| predicate())
    }

    pub fn try_every<F>(&self, mut predicate: F) -> TsonicResult<bool>
    where
        T: Clone,
        F: FnMut(T) -> TsonicResult<bool>,
    {
        self.try_quantify_with(true, |value, _, _| predicate(value))
    }

    pub fn try_every_with_index<F>(&self, mut predicate: F) -> TsonicResult<bool>
    where
        T: Clone,
        F: FnMut(T, f64) -> TsonicResult<bool>,
    {
        self.try_quantify_with(true, |value, index, _| predicate(value, index))
    }

    pub fn try_every_with_array<F>(&self, predicate: F) -> TsonicResult<bool>
    where
        T: Clone,
        F: FnMut(T, f64, Self) -> TsonicResult<bool>,
    {
        self.try_quantify_with(true, predicate)
    }
}
