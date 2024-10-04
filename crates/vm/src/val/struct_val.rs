use std::collections::HashMap;

use super::{Symbol, UnsafeVal, Val};

/// A container for a struct. A struct is a map from name to value.
#[derive(Clone, Debug, Default)]
pub struct StructVal {
    inner: HashMap<Symbol, UnsafeVal>,
}

impl StructVal {
    /// Create a new `StructVal`.
    pub fn new() -> StructVal {
        StructVal::with_capacity(0)
    }

    /// Create a new `StructVal` with capacity for `cap` amount of fields.
    pub fn with_capacity(cap: usize) -> StructVal {
        StructVal {
            inner: HashMap::with_capacity(cap),
        }
    }

    /// Get the number of fields.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns `true` if the struct is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Set the value of `symbol` to `value`.
    pub fn set(&mut self, symbol: Symbol, value: Val<'static>) {
        self.inner.insert(symbol, value.inner);
    }

    /// Get the value of `symbol` or `None` if the value does not exist.
    pub fn get(&self, symbol: Symbol) -> Option<UnsafeVal> {
        self.inner.get(&symbol).copied()
    }

    /// Iterate over all symbol,values within `self`.
    pub fn iter(&self) -> impl '_ + Iterator<Item = (Symbol, UnsafeVal)> {
        self.inner.iter().map(|(k, v)| (*k, *v))
    }

    /// Iterate over all values within `self`.
    pub fn values(&self) -> impl '_ + Iterator<Item = UnsafeVal> {
        self.inner.values().copied()
    }
}
