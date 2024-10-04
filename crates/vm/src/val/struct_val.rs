use std::collections::HashMap;

use crate::Vm;

use super::{Symbol, UnsafeVal};

/// A container for a struct. A struct is a map from name to value.
#[derive(Clone, Debug, Default)]
pub struct StructVal {
    inner: HashMap<Symbol, UnsafeVal>,
}

impl StructVal {
    pub fn new() -> StructVal {
        StructVal::with_capacity(0)
    }

    pub fn with_capacity(cap: usize) -> StructVal {
        StructVal {
            inner: HashMap::with_capacity(cap),
        }
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn set_with_name(&mut self, vm: &mut Vm, name: &str, value: UnsafeVal) {
        let symbol = vm.get_or_create_symbol(name);
        self.set(symbol, value);
    }

    pub fn set(&mut self, symbol: Symbol, value: UnsafeVal) {
        self.inner.insert(symbol, value);
    }

    pub fn get(&self, symbol: Symbol) -> Option<UnsafeVal> {
        self.inner.get(&symbol).copied()
    }

    pub fn get_with_name(&self, vm: &Vm, name: &str) -> Option<UnsafeVal> {
        let symbol = vm.get_symbol(name)?;
        self.get(symbol)
    }

    pub fn iter(&self) -> impl '_ + Iterator<Item = (Symbol, UnsafeVal)> {
        self.inner.iter().map(|(k, v)| (*k, *v))
    }

    pub fn values(&self) -> impl '_ + Iterator<Item = UnsafeVal> {
        self.inner.values().copied()
    }
}
