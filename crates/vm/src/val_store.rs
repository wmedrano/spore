use std::{collections::HashMap, marker::PhantomData, sync::Arc};

use crate::val::{ByteCode, InternalVal};

/// A unique identifier for an object in `ValStore`.
#[derive(Debug)]
pub struct ValId<T> {
    id: u32,
    _marker: PhantomData<T>,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
enum Color {
    #[default]
    Red,
    Blue,
}

impl Color {
    pub fn swap(self) -> Color {
        match self {
            Color::Red => Color::Blue,
            Color::Blue => Color::Red,
        }
    }
}

#[derive(Clone, Debug)]
struct ValWithColor<T> {
    inner: T,
    color: Color,
    keep_alive_count: u32,
}

/// ValStore manages the lifetime of Val objects.
#[derive(Clone, Debug, Default)]
pub struct ValStore {
    strings: HashMap<ValId<String>, ValWithColor<String>>,
    bytecodes: HashMap<ValId<Arc<ByteCode>>, ValWithColor<Arc<ByteCode>>>,
    next_string: u32,
    next_bytecode: u32,
    alive_color: Color,
}

impl ValStore {
    /// Run the garbage collector. All known values must be in `values`.
    pub fn run_gc(&mut self, values: impl Iterator<Item = InternalVal>) {
        let self_ptr: *mut ValStore = self;
        // 1. Mark.
        for val in values {
            match val {
                InternalVal::Void => {}
                InternalVal::Bool(_) => {}
                InternalVal::Int(_) => {}
                InternalVal::Float(_) => {}
                InternalVal::String(id) => {
                    if let Some(entry) = self.strings.get_mut(&id) {
                        entry.color = self.alive_color;
                    }
                }
                InternalVal::ByteCodeFunction(id) => {
                    if let Some(entry) = self.bytecodes.get_mut(&id) {
                        if entry.color != self.alive_color {
                            entry.color = self.alive_color;
                            let child_values = entry.inner.values();
                            unsafe { &mut *self_ptr }.run_gc(child_values);
                        }
                    }
                }
                InternalVal::NativeFunction(_) => {}
            }
        }
        // 2. Sweep.
        self.strings.retain(|_, v| v.color == self.alive_color);
        self.bytecodes.retain(|_, v| v.color == self.alive_color);
    }

    pub fn keep_alive(&mut self, value: InternalVal) {
        match value {
            InternalVal::Void => {}
            InternalVal::Bool(_) => {}
            InternalVal::Int(_) => {}
            InternalVal::Float(_) => {}
            InternalVal::String(id) => {
                if let Some(s) = self.strings.get_mut(&id) {
                    s.keep_alive_count += 1;
                }
            }
            InternalVal::ByteCodeFunction(id) => {
                if let Some(bc) = self.bytecodes.get_mut(&id) {
                    bc.keep_alive_count += 1;
                }
            }
            InternalVal::NativeFunction(_) => {}
        }
    }

    pub fn allow_death(&mut self, value: InternalVal) {
        match value {
            InternalVal::Void => {}
            InternalVal::Bool(_) => {}
            InternalVal::Int(_) => {}
            InternalVal::Float(_) => {}
            InternalVal::String(id) => {
                if let Some(s) = self.strings.get_mut(&id) {
                    s.keep_alive_count -= s.keep_alive_count.saturating_sub(1);
                }
            }
            InternalVal::ByteCodeFunction(id) => {
                if let Some(bc) = self.bytecodes.get_mut(&id) {
                    bc.keep_alive_count -= bc.keep_alive_count.saturating_sub(1);
                }
            }
            InternalVal::NativeFunction(_) => {}
        }
    }

    /// Get a string by its id.
    pub fn get_str<'a>(&'a self, id: ValId<String>) -> &'a str {
        let res = self.strings.get(&id);
        debug_assert!(res.is_some());
        res.map(|s| s.inner.as_str()).unwrap_or("")
    }

    /// Insert a string and get its id.
    pub fn insert_string(&mut self, s: String) -> ValId<String> {
        let id = ValId::new(self.next_string);
        self.next_string += 1;
        self.strings.insert(
            id,
            ValWithColor {
                inner: s,
                color: self.alive_color,
                keep_alive_count: 0,
            },
        );
        id
    }

    /// Get a bytecode by its id.
    pub fn get_bytecode<'a>(&'a self, id: ValId<Arc<ByteCode>>) -> &'a Arc<ByteCode> {
        let res = self.bytecodes.get(&id);
        debug_assert!(res.is_some());
        res.map(|bc| &bc.inner).unwrap()
    }

    /// Get bytecode id for any bytecode that is equal to `bytecode`. If it does not exist, then it
    /// is inserted into the map.
    ///
    /// Warning: This may be very slow.
    #[cfg(test)]
    pub fn get_or_insert_bytecode_slow(&mut self, bytecode: ByteCode) -> ValId<Arc<ByteCode>> {
        for (k, v) in self.bytecodes.iter() {
            if bytecode == *v.inner {
                return *k;
            }
        }
        self.insert_bytecode(bytecode)
    }

    /// Insert bytecode into the store and return its id.
    pub fn insert_bytecode(&mut self, bytecode: ByteCode) -> ValId<Arc<ByteCode>> {
        let id = ValId::new(self.next_bytecode);
        self.next_bytecode += 1;
        self.bytecodes.insert(
            id,
            ValWithColor {
                inner: bytecode.into(),
                // Consider the value dead so that we have to traverse its children during the next
                // mark.
                color: self.alive_color.swap(),
                keep_alive_count: 0,
            },
        );
        id
    }
}

impl<T> PartialEq for ValId<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl<T> Eq for ValId<T> {}
impl<T> Copy for ValId<T> {}
impl<T> Clone for ValId<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> std::hash::Hash for ValId<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl<T> ValId<T> {
    pub fn new(id: u32) -> ValId<T> {
        ValId {
            id,
            _marker: PhantomData,
        }
    }
}
