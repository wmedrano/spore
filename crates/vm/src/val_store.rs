use std::{marker::PhantomData, sync::Arc};

use crate::val::{bytecode::ByteCode, InternalVal};

type IdRepr = u32;

/// A unique identifier for an object in `ValStore`.
#[derive(Debug, Default)]
pub struct ValId<T> {
    id: IdRepr,
    _marker: PhantomData<T>,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
enum Color {
    #[default]
    Red,
    Blue,
    Tombstone,
}

impl Color {
    pub fn swap(self) -> Color {
        match self {
            Color::Red => Color::Blue,
            Color::Blue => Color::Red,
            Color::Tombstone => Color::Tombstone,
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
    strings: Vec<ValWithColor<String>>,
    bytecodes: Vec<ValWithColor<Option<Arc<ByteCode>>>>,
    free_string_ids: Vec<ValId<String>>,
    free_bytecode_ids: Vec<ValId<Arc<ByteCode>>>,
    alive_color: Color,
}

impl ValStore {
    /// Run the garbage collector. All known values must be in `values`.
    pub fn run_gc(&mut self, values: impl Iterator<Item = InternalVal>) {
        let self_ptr: *mut ValStore = self;
        let mark_bytecode_child_vals = move |bc: &ByteCode| {
            unsafe { &mut *self_ptr }.run_gc(bc.values());
        };
        // 1. Mark.
        for val in values {
            match val {
                InternalVal::Void => {}
                InternalVal::Bool(_) => {}
                InternalVal::Int(_) => {}
                InternalVal::Float(_) => {}
                InternalVal::String(id) => {
                    if let Some(entry) = self.strings.get_mut(id.id as usize) {
                        entry.color = self.alive_color;
                    }
                }
                InternalVal::ByteCodeFunction(id) => {
                    if let Some(entry) = self.bytecodes.get_mut(id.id as usize) {
                        if let Some(bc) = entry.inner.as_ref() {
                            if entry.color != self.alive_color {
                                entry.color = self.alive_color;
                                mark_bytecode_child_vals(bc);
                            }
                        }
                    }
                }
                InternalVal::NativeFunction(_) => {}
            }
        }
        for entry in self.bytecodes.iter_mut() {
            if let Some(bc) = entry.inner.as_ref() {
                if entry.keep_alive_count > 0 && entry.color != self.alive_color {
                    entry.color = self.alive_color;
                    mark_bytecode_child_vals(bc);
                }
            }
        }
        // 2. Sweep.
        for (idx, string) in self
            .strings
            .iter_mut()
            .enumerate()
            .filter(|(_, s)| s.color != self.alive_color && s.color != Color::Tombstone)
            .filter(|(_, s)| s.keep_alive_count == 0)
        {
            *string = ValWithColor {
                inner: String::new(),
                color: Color::Tombstone,
                keep_alive_count: 0,
            };
            self.free_string_ids.push(ValId::new(idx as u32));
        }
        for (idx, bc) in self
            .bytecodes
            .iter_mut()
            .enumerate()
            .filter(|(_, x)| x.color != self.alive_color && x.color != Color::Tombstone)
            .filter(|(_, x)| x.keep_alive_count == 0)
        {
            *bc = ValWithColor {
                inner: None,
                color: Color::Tombstone,
                keep_alive_count: 0,
            };
            self.free_bytecode_ids.push(ValId::new(idx as u32));
        }
    }

    pub fn keep_alive(&mut self, value: InternalVal) {
        match value {
            InternalVal::Void => {}
            InternalVal::Bool(_) => {}
            InternalVal::Int(_) => {}
            InternalVal::Float(_) => {}
            InternalVal::String(id) => {
                if let Some(s) = self.strings.get_mut(id.id as usize) {
                    s.keep_alive_count += 1;
                }
            }
            InternalVal::ByteCodeFunction(id) => {
                if let Some(bc) = self.bytecodes.get_mut(id.id as usize) {
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
                if let Some(s) = self.strings.get_mut(id.id as usize) {
                    s.keep_alive_count -= s.keep_alive_count.saturating_sub(1);
                }
            }
            InternalVal::ByteCodeFunction(id) => {
                if let Some(bc) = self.bytecodes.get_mut(id.id as usize) {
                    bc.keep_alive_count -= bc.keep_alive_count.saturating_sub(1);
                }
            }
            InternalVal::NativeFunction(_) => {}
        }
    }

    /// Get a string by its id.
    pub fn get_str(&self, id: ValId<String>) -> &str {
        let res = self.strings.get(id.id as usize);
        debug_assert!(res.is_some());
        res.map(|s| s.inner.as_str()).unwrap_or("")
    }

    /// Insert a string and get its id.
    pub fn insert_string(&mut self, s: String) -> ValId<String> {
        let val = ValWithColor {
            inner: s,
            color: self.alive_color,
            keep_alive_count: 0,
        };
        match self.free_string_ids.pop() {
            Some(id) => {
                self.strings[id.id as usize] = val;
                id
            }
            None => {
                let id = ValId::new(self.strings.len() as u32);
                self.strings.push(val);
                id
            }
        }
    }

    /// Get a bytecode by its id.
    pub fn get_bytecode(&self, id: ValId<Arc<ByteCode>>) -> &Arc<ByteCode> {
        let res = self.bytecodes.get(id.id as usize);
        match res {
            Some(ValWithColor {
                inner: Some(bc), ..
            }) => bc,
            _ => panic!("{id:?} not found."),
        }
    }

    /// Get bytecode id for any bytecode that is equal to `bytecode`. If it does not exist, then it
    /// is inserted into the map.
    ///
    /// Warning: This may be very slow.
    #[cfg(test)]
    pub fn get_or_insert_bytecode_slow(&mut self, bytecode: ByteCode) -> ValId<Arc<ByteCode>> {
        for (idx, val) in self.bytecodes.iter().enumerate() {
            if val
                .inner
                .as_ref()
                .map(|bc| bc.as_ref() == &bytecode)
                .unwrap_or(false)
            {
                return ValId::new(idx as u32);
            }
        }
        self.insert_bytecode(bytecode)
    }

    /// Insert bytecode into the store and return its id.
    pub fn insert_bytecode(&mut self, bytecode: ByteCode) -> ValId<Arc<ByteCode>> {
        let val = ValWithColor {
            inner: Some(bytecode.into()),
            // Consider the value dead so that we have to traverse its children during the next
            // mark.
            color: self.alive_color.swap(),
            keep_alive_count: 0,
        };
        match self.free_bytecode_ids.pop() {
            Some(id) => {
                self.bytecodes[id.id as usize] = val;
                id
            }
            None => {
                let id = ValId::new(self.bytecodes.len() as u32);
                self.bytecodes.push(val);
                id
            }
        }
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
        state.write(&self.id.to_ne_bytes());
    }
}

impl<T> ValId<T> {
    pub fn new(id: IdRepr) -> ValId<T> {
        ValId {
            id,
            _marker: PhantomData,
        }
    }
}
