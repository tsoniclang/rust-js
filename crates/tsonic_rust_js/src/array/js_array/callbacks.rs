use super::JsArray;
use tsonic_rust_runtime::{JsError, JsErrorKind};

impl<T> JsArray<T> {
    fn map_with<U, F>(&self, mut mapper: F) -> JsArray<U>
    where
        T: Clone,
        F: FnMut(T, usize, Self) -> U,
    {
        let length = self.len();
        let output = JsArray::with_capacity(length);
        for index in 0..length {
            let value = self
                .get(index)
                .expect("Array.map cannot create holes after its source is shortened");
            output.push(mapper(value, index, self.clone()));
        }
        output
    }

    pub fn map_zero<U, F>(&self, mut mapper: F) -> JsArray<U>
    where
        T: Clone,
        F: FnMut() -> U,
    {
        self.map_with(|_, _, _| mapper())
    }

    pub fn map<U, F>(&self, mut mapper: F) -> JsArray<U>
    where
        T: Clone,
        F: FnMut(T) -> U,
    {
        self.map_with(|value, _, _| mapper(value))
    }

    pub fn map_with_index<U, F>(&self, mut mapper: F) -> JsArray<U>
    where
        T: Clone,
        F: FnMut(T, usize) -> U,
    {
        self.map_with(|value, index, _| mapper(value, index))
    }

    pub fn map_with_array<U, F>(&self, mapper: F) -> JsArray<U>
    where
        T: Clone,
        F: FnMut(T, usize, Self) -> U,
    {
        self.map_with(mapper)
    }

    fn filter_with<F>(&self, mut predicate: F) -> Self
    where
        T: Clone,
        F: FnMut(T, usize, Self) -> bool,
    {
        let length = self.len();
        let output = Self::new();
        for index in 0..length {
            if let Some(value) = self.get(index) {
                if predicate(value.clone(), index, self.clone()) {
                    output.push(value);
                }
            }
        }
        output
    }

    pub fn filter_zero<F>(&self, mut predicate: F) -> Self
    where
        T: Clone,
        F: FnMut() -> bool,
    {
        self.filter_with(|_, _, _| predicate())
    }

    pub fn filter<F>(&self, mut predicate: F) -> Self
    where
        T: Clone,
        F: FnMut(T) -> bool,
    {
        self.filter_with(|value, _, _| predicate(value))
    }

    pub fn filter_with_index<F>(&self, mut predicate: F) -> Self
    where
        T: Clone,
        F: FnMut(T, usize) -> bool,
    {
        self.filter_with(|value, index, _| predicate(value, index))
    }

    pub fn filter_with_array<F>(&self, predicate: F) -> Self
    where
        T: Clone,
        F: FnMut(T, usize, Self) -> bool,
    {
        self.filter_with(predicate)
    }

    fn reduce_with<U, F>(&self, initial: U, mut reducer: F) -> U
    where
        T: Clone,
        F: FnMut(U, T, usize, Self) -> U,
    {
        let length = self.len();
        let mut accumulator = initial;
        for index in 0..length {
            if let Some(value) = self.get(index) {
                accumulator = reducer(accumulator, value, index, self.clone());
            }
        }
        accumulator
    }

    pub fn reduce_zero<U, F>(&self, initial: U, mut reducer: F) -> U
    where
        T: Clone,
        F: FnMut() -> U,
    {
        self.reduce_with(initial, |_, _, _, _| reducer())
    }

    pub fn reduce_accumulator<U, F>(&self, initial: U, mut reducer: F) -> U
    where
        T: Clone,
        F: FnMut(U) -> U,
    {
        self.reduce_with(initial, |accumulator, _, _, _| reducer(accumulator))
    }

    pub fn reduce<U, F>(&self, initial: U, mut reducer: F) -> U
    where
        T: Clone,
        F: FnMut(U, T) -> U,
    {
        self.reduce_with(initial, |accumulator, value, _, _| {
            reducer(accumulator, value)
        })
    }

    pub fn reduce_with_index<U, F>(&self, initial: U, mut reducer: F) -> U
    where
        T: Clone,
        F: FnMut(U, T, usize) -> U,
    {
        self.reduce_with(initial, |accumulator, value, index, _| {
            reducer(accumulator, value, index)
        })
    }

    pub fn reduce_with_array<U, F>(&self, initial: U, reducer: F) -> U
    where
        T: Clone,
        F: FnMut(U, T, usize, Self) -> U,
    {
        self.reduce_with(initial, reducer)
    }

    fn reduce_from_first_with<F>(&self, mut reducer: F) -> Result<T, JsError>
    where
        T: Clone,
        F: FnMut(T, T, usize, Self) -> T,
    {
        let length = self.len();
        let Some((first_index, mut accumulator)) =
            (0..length).find_map(|index| self.get(index).map(|value| (index, value)))
        else {
            return Err(JsError::new(
                JsErrorKind::TypeError,
                "Reduce of empty array with no initial value",
            ));
        };
        for index in first_index + 1..length {
            if let Some(value) = self.get(index) {
                accumulator = reducer(accumulator, value, index, self.clone());
            }
        }
        Ok(accumulator)
    }

    pub fn reduce_from_first_zero<F>(&self, mut reducer: F) -> Result<T, JsError>
    where
        T: Clone,
        F: FnMut() -> T,
    {
        self.reduce_from_first_with(|_, _, _, _| reducer())
    }

    pub fn reduce_from_first_accumulator<F>(&self, mut reducer: F) -> Result<T, JsError>
    where
        T: Clone,
        F: FnMut(T) -> T,
    {
        self.reduce_from_first_with(|accumulator, _, _, _| reducer(accumulator))
    }

    pub fn reduce_from_first<F>(&self, mut reducer: F) -> Result<T, JsError>
    where
        T: Clone,
        F: FnMut(T, T) -> T,
    {
        self.reduce_from_first_with(|accumulator, value, _, _| reducer(accumulator, value))
    }

    pub fn reduce_from_first_with_index<F>(&self, mut reducer: F) -> Result<T, JsError>
    where
        T: Clone,
        F: FnMut(T, T, usize) -> T,
    {
        self.reduce_from_first_with(|accumulator, value, index, _| {
            reducer(accumulator, value, index)
        })
    }

    pub fn reduce_from_first_with_array<F>(&self, reducer: F) -> Result<T, JsError>
    where
        T: Clone,
        F: FnMut(T, T, usize, Self) -> T,
    {
        self.reduce_from_first_with(reducer)
    }

    pub fn for_each_zero<F>(&self, mut callback: F)
    where
        T: Clone,
        F: FnMut(),
    {
        self.for_each_with(|_, _, _| callback());
    }

    pub fn for_each_value<F>(&self, mut callback: F)
    where
        T: Clone,
        F: FnMut(T),
    {
        self.for_each_with(|value, _, _| callback(value));
    }

    pub fn for_each_value_index<F>(&self, mut callback: F)
    where
        T: Clone,
        F: FnMut(T, usize),
    {
        self.for_each_with(|value, index, _| callback(value, index));
    }

    pub fn for_each<F>(&self, callback: F)
    where
        T: Clone,
        F: FnMut(T, usize, Self),
    {
        self.for_each_with(callback);
    }

    fn for_each_with<F>(&self, mut callback: F)
    where
        T: Clone,
        F: FnMut(T, usize, Self),
    {
        let length = self.len();
        for index in 0..length {
            if let Some(value) = self.get(index) {
                callback(value, index, self.clone());
            }
        }
    }

    fn find_with<F>(&self, mut predicate: F) -> Option<T>
    where
        T: Clone,
        F: FnMut(T, usize, Self) -> bool,
    {
        let length = self.len();
        for index in 0..length {
            if let Some(value) = self.get(index) {
                if predicate(value.clone(), index, self.clone()) {
                    return Some(value);
                }
            }
        }
        None
    }

    pub fn find_zero<F>(&self, mut predicate: F) -> Option<T>
    where
        T: Clone,
        F: FnMut() -> bool,
    {
        self.find_with(|_, _, _| predicate())
    }

    pub fn find<F>(&self, mut predicate: F) -> Option<T>
    where
        T: Clone,
        F: FnMut(T) -> bool,
    {
        self.find_with(|value, _, _| predicate(value))
    }

    pub fn find_with_index<F>(&self, mut predicate: F) -> Option<T>
    where
        T: Clone,
        F: FnMut(T, usize) -> bool,
    {
        self.find_with(|value, index, _| predicate(value, index))
    }

    pub fn find_with_array<F>(&self, predicate: F) -> Option<T>
    where
        T: Clone,
        F: FnMut(T, usize, Self) -> bool,
    {
        self.find_with(predicate)
    }

    fn find_index_with<F>(&self, mut predicate: F) -> isize
    where
        T: Clone,
        F: FnMut(T, usize, Self) -> bool,
    {
        let length = self.len();
        for index in 0..length {
            if let Some(value) = self.get(index) {
                if predicate(value, index, self.clone()) {
                    return index as isize;
                }
            }
        }
        -1
    }

    pub fn find_index_zero<F>(&self, mut predicate: F) -> isize
    where
        T: Clone,
        F: FnMut() -> bool,
    {
        self.find_index_with(|_, _, _| predicate())
    }

    pub fn find_index<F>(&self, mut predicate: F) -> isize
    where
        T: Clone,
        F: FnMut(T) -> bool,
    {
        self.find_index_with(|value, _, _| predicate(value))
    }

    pub fn find_index_with_index<F>(&self, mut predicate: F) -> isize
    where
        T: Clone,
        F: FnMut(T, usize) -> bool,
    {
        self.find_index_with(|value, index, _| predicate(value, index))
    }

    pub fn find_index_with_array<F>(&self, predicate: F) -> isize
    where
        T: Clone,
        F: FnMut(T, usize, Self) -> bool,
    {
        self.find_index_with(predicate)
    }

    fn find_last_with<F>(&self, mut predicate: F) -> Option<T>
    where
        T: Clone,
        F: FnMut(T, usize, Self) -> bool,
    {
        for index in (0..self.len()).rev() {
            if let Some(value) = self.get(index) {
                if predicate(value.clone(), index, self.clone()) {
                    return Some(value);
                }
            }
        }
        None
    }

    pub fn find_last_zero<F>(&self, mut predicate: F) -> Option<T>
    where
        T: Clone,
        F: FnMut() -> bool,
    {
        self.find_last_with(|_, _, _| predicate())
    }

    pub fn find_last<F>(&self, mut predicate: F) -> Option<T>
    where
        T: Clone,
        F: FnMut(T) -> bool,
    {
        self.find_last_with(|value, _, _| predicate(value))
    }

    pub fn find_last_with_index<F>(&self, mut predicate: F) -> Option<T>
    where
        T: Clone,
        F: FnMut(T, usize) -> bool,
    {
        self.find_last_with(|value, index, _| predicate(value, index))
    }

    pub fn find_last_with_array<F>(&self, predicate: F) -> Option<T>
    where
        T: Clone,
        F: FnMut(T, usize, Self) -> bool,
    {
        self.find_last_with(predicate)
    }

    fn find_last_index_with<F>(&self, mut predicate: F) -> isize
    where
        T: Clone,
        F: FnMut(T, usize, Self) -> bool,
    {
        for index in (0..self.len()).rev() {
            if let Some(value) = self.get(index) {
                if predicate(value, index, self.clone()) {
                    return index as isize;
                }
            }
        }
        -1
    }

    pub fn find_last_index_zero<F>(&self, mut predicate: F) -> isize
    where
        T: Clone,
        F: FnMut() -> bool,
    {
        self.find_last_index_with(|_, _, _| predicate())
    }

    pub fn find_last_index<F>(&self, mut predicate: F) -> isize
    where
        T: Clone,
        F: FnMut(T) -> bool,
    {
        self.find_last_index_with(|value, _, _| predicate(value))
    }

    pub fn find_last_index_with_index<F>(&self, mut predicate: F) -> isize
    where
        T: Clone,
        F: FnMut(T, usize) -> bool,
    {
        self.find_last_index_with(|value, index, _| predicate(value, index))
    }

    pub fn find_last_index_with_array<F>(&self, predicate: F) -> isize
    where
        T: Clone,
        F: FnMut(T, usize, Self) -> bool,
    {
        self.find_last_index_with(predicate)
    }

    fn some_with<F>(&self, mut predicate: F) -> bool
    where
        T: Clone,
        F: FnMut(T, usize, Self) -> bool,
    {
        let length = self.len();
        (0..length).any(|index| {
            self.get(index)
                .is_some_and(|value| predicate(value, index, self.clone()))
        })
    }

    pub fn some_zero<F>(&self, mut predicate: F) -> bool
    where
        T: Clone,
        F: FnMut() -> bool,
    {
        self.some_with(|_, _, _| predicate())
    }

    pub fn some<F>(&self, mut predicate: F) -> bool
    where
        T: Clone,
        F: FnMut(T) -> bool,
    {
        self.some_with(|value, _, _| predicate(value))
    }

    pub fn some_with_index<F>(&self, mut predicate: F) -> bool
    where
        T: Clone,
        F: FnMut(T, usize) -> bool,
    {
        self.some_with(|value, index, _| predicate(value, index))
    }

    pub fn some_with_array<F>(&self, predicate: F) -> bool
    where
        T: Clone,
        F: FnMut(T, usize, Self) -> bool,
    {
        self.some_with(predicate)
    }

    fn every_with<F>(&self, mut predicate: F) -> bool
    where
        T: Clone,
        F: FnMut(T, usize, Self) -> bool,
    {
        let length = self.len();
        (0..length).all(|index| {
            self.get(index)
                .is_none_or(|value| predicate(value, index, self.clone()))
        })
    }

    pub fn every_zero<F>(&self, mut predicate: F) -> bool
    where
        T: Clone,
        F: FnMut() -> bool,
    {
        self.every_with(|_, _, _| predicate())
    }

    pub fn every<F>(&self, mut predicate: F) -> bool
    where
        T: Clone,
        F: FnMut(T) -> bool,
    {
        self.every_with(|value, _, _| predicate(value))
    }

    pub fn every_with_index<F>(&self, mut predicate: F) -> bool
    where
        T: Clone,
        F: FnMut(T, usize) -> bool,
    {
        self.every_with(|value, index, _| predicate(value, index))
    }

    pub fn every_with_array<F>(&self, predicate: F) -> bool
    where
        T: Clone,
        F: FnMut(T, usize, Self) -> bool,
    {
        self.every_with(predicate)
    }

}
