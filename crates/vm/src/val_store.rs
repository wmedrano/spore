use std::{marker::PhantomData, sync::Arc};

use smol_str::SmolStr;

use crate::val::{internal::InternalValImpl, ByteCode, InternalVal, ListVal};

type IdRepr = u32;

/// A unique identifier for an object in `ValStore`.
#[derive(Debug, Default)]
pub struct ValId<T> {
    id: IdRepr,
    _marker: PhantomData<T>,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
enum Color {
    /// A color that may mark if a value is used or unused.
    #[default]
    Red,
    /// A color that may mark if a value is used or unused.
    Blue,
    /// Denotes that the value does not exist.
    Tombstone,
}

impl Color {
    /// Swaps `Red` and `Blue`. `Tombstone` is returned unchanged.
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
    strings: Vec<ValWithColor<SmolStr>>,
    lists: Vec<ValWithColor<ListVal>>,
    bytecodes: Vec<ValWithColor<Option<Arc<ByteCode>>>>,
    free_string_ids: Vec<ValId<SmolStr>>,
    free_list_ids: Vec<ValId<ListVal>>,
    free_bytecode_ids: Vec<ValId<Arc<ByteCode>>>,
    alive_color: Color,
    // Data used for GC mark phase.
    temp_mark_data: TempMarkData,
}

#[derive(Clone, Debug, Default)]
struct TempMarkData {
    current_queue: Vec<InternalVal>,
    next_queue: Vec<InternalVal>,
}

impl ValStore {
    /// Run the garbage collector. All known values must be in `values`.
    pub fn run_gc(&mut self, values: impl Iterator<Item = InternalVal>) {
        let mut temp_data = std::mem::take(&mut self.temp_mark_data);
        self.run_gc_mark(&mut temp_data, values);
        self.temp_mark_data = temp_data;
        self.run_gc_sweep();
        self.alive_color = self.alive_color.swap();
    }

    /// Run the GC mark phase.
    fn run_gc_mark(
        &mut self,
        temp_data: &mut TempMarkData,
        values: impl Iterator<Item = InternalVal>,
    ) {
        self.init_gc_mark(temp_data, values);
        while !temp_data.current_queue.is_empty() {
            for val in temp_data.current_queue.drain(..) {
                self.gc_mark_one(val, &mut temp_data.next_queue);
            }
            std::mem::swap(&mut temp_data.current_queue, &mut temp_data.next_queue);
        }
    }

    /// Initialize the GC mark phase. This takes `values` and enqueues them for marking in
    /// `temp_data.current_queue`.
    fn init_gc_mark(
        &self,
        temp_data: &mut TempMarkData,
        values: impl Iterator<Item = InternalVal>,
    ) {
        temp_data.clear_retaining_capacity();
        for val in values {
            if is_garbage_collected(val) {
                temp_data.current_queue.push(val);
            }
        }
        for (idx, colored_vals) in self.strings.iter().enumerate() {
            if colored_vals.keep_alive_count > 0 {
                temp_data
                    .current_queue
                    .push(InternalValImpl::String(ValId::new(idx as u32)).into());
            }
        }
        for (idx, colored_vals) in self.lists.iter().enumerate() {
            if colored_vals.keep_alive_count > 0 {
                temp_data
                    .current_queue
                    .push(InternalValImpl::List(ValId::new(idx as u32)).into());
            }
        }
        for (idx, colored_vals) in self.bytecodes.iter().enumerate() {
            if colored_vals.keep_alive_count > 0 {
                temp_data
                    .current_queue
                    .push(InternalValImpl::ByteCodeFunction(ValId::new(idx as u32)).into());
            }
        }
    }

    fn gc_mark_one(&mut self, val: InternalVal, child_queue: &mut Vec<InternalVal>) {
        let mut add_child = |v| {
            if is_garbage_collected(v) {
                child_queue.push(v);
            }
        };
        match val.0 {
            InternalValImpl::String(id) => {
                if let Some(entry) = self.strings.get_mut(id.id as usize) {
                    entry.color = self.alive_color;
                }
            }
            InternalValImpl::List(id) => {
                if let Some(entry) = self.lists.get_mut(id.id as usize) {
                    if entry.color != self.alive_color {
                        debug_assert_ne!(entry.color, Color::Tombstone);
                        entry.color = self.alive_color;
                        for child_val in entry.inner.iter() {
                            add_child(*child_val);
                        }
                    }
                }
            }
            InternalValImpl::ByteCodeFunction(id) => {
                if let Some(entry) = self.bytecodes.get_mut(id.id as usize) {
                    if let Some(bc) = entry.inner.as_ref() {
                        if entry.color != self.alive_color {
                            entry.color = self.alive_color;
                            for child_val in bc.values() {
                                add_child(child_val);
                            }
                        }
                    }
                }
            }
            _ => assert!(!is_garbage_collected(val)),
        }
    }

    fn run_gc_sweep(&mut self) {
        for (idx, string) in self
            .strings
            .iter_mut()
            .enumerate()
            .filter(|(_, s)| s.color != self.alive_color && s.color != Color::Tombstone)
            .filter(|(_, s)| s.keep_alive_count == 0)
        {
            *string = ValWithColor {
                inner: Default::default(),
                color: Color::Tombstone,
                keep_alive_count: 0,
            };
            self.free_string_ids.push(ValId::new(idx as u32));
        }
        for (idx, list) in self
            .lists
            .iter_mut()
            .enumerate()
            .filter(|(_, s)| s.color != self.alive_color && s.color != Color::Tombstone)
            .filter(|(_, s)| s.keep_alive_count == 0)
        {
            *list = ValWithColor {
                inner: Vec::new(),
                color: Color::Tombstone,
                keep_alive_count: 0,
            };
            self.free_list_ids.push(ValId::new(idx as u32));
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
        match value.0 {
            InternalValImpl::Void => {}
            InternalValImpl::Bool(_) => {}
            InternalValImpl::Int(_) => {}
            InternalValImpl::Float(_) => {}
            InternalValImpl::String(id) => {
                if let Some(s) = self.strings.get_mut(id.id as usize) {
                    s.keep_alive_count += 1;
                }
            }
            InternalValImpl::List(id) => {
                if let Some(s) = self.lists.get_mut(id.id as usize) {
                    s.keep_alive_count += 1;
                }
            }
            InternalValImpl::ByteCodeFunction(id) => {
                if let Some(bc) = self.bytecodes.get_mut(id.id as usize) {
                    bc.keep_alive_count += 1;
                }
            }
            InternalValImpl::NativeFunction(_) => {}
        }
    }

    pub fn allow_death(&mut self, value: InternalVal) {
        match value.0 {
            InternalValImpl::Void => {}
            InternalValImpl::Bool(_) => {}
            InternalValImpl::Int(_) => {}
            InternalValImpl::Float(_) => {}
            InternalValImpl::String(id) => {
                if let Some(s) = self.strings.get_mut(id.id as usize) {
                    s.keep_alive_count -= s.keep_alive_count.saturating_sub(1);
                }
            }
            InternalValImpl::List(id) => {
                if let Some(s) = self.lists.get_mut(id.id as usize) {
                    s.keep_alive_count -= s.keep_alive_count.saturating_sub(1);
                }
            }
            InternalValImpl::ByteCodeFunction(id) => {
                if let Some(bc) = self.bytecodes.get_mut(id.id as usize) {
                    bc.keep_alive_count -= bc.keep_alive_count.saturating_sub(1);
                }
            }
            InternalValImpl::NativeFunction(_) => {}
        }
    }

    /// Get a string by its id.
    pub fn get_str(&self, id: ValId<SmolStr>) -> &str {
        let res = self.strings.get(id.id as usize);
        debug_assert!(res.is_some());
        res.map(|s| s.inner.as_str()).unwrap_or("")
    }

    /// Insert a string and get its id.
    pub fn insert_string(&mut self, s: SmolStr) -> ValId<SmolStr> {
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

    pub const EMPTY_LIST: &ListVal = &ListVal::new();

    /// Get a list by its id.
    pub fn get_list(&self, id: ValId<ListVal>) -> &ListVal {
        let res = self.lists.get(id.id as usize);
        debug_assert!(res.is_some(), "{id:?} not found.");
        res.map(|s| &s.inner).unwrap_or(Self::EMPTY_LIST)
    }

    /// Insert a list and get its id.
    pub fn insert_list(&mut self, list: ListVal) -> ValId<ListVal> {
        let val = ValWithColor {
            inner: list,
            color: self.alive_color,
            keep_alive_count: 0,
        };
        match self.free_list_ids.pop() {
            Some(id) => {
                self.lists[id.id as usize] = val;
                id
            }
            None => {
                let id = ValId::new(self.lists.len() as u32);
                self.lists.push(val);
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

/// Returns `true` if `v` is managed by the garbage collector.
fn is_garbage_collected(v: InternalVal) -> bool {
    match v.0 {
        InternalValImpl::Void => false,
        InternalValImpl::Bool(_) => false,
        InternalValImpl::Int(_) => false,
        InternalValImpl::Float(_) => false,
        InternalValImpl::String(_) => true,
        InternalValImpl::List(_) => true,
        InternalValImpl::ByteCodeFunction(_) => true,
        InternalValImpl::NativeFunction(_) => false,
    }
}

impl TempMarkData {
    fn clear_retaining_capacity(&mut self) {
        self.current_queue.clear();
        self.next_queue.clear();
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
impl<T> ValId<T> {
    pub fn new(id: IdRepr) -> ValId<T> {
        ValId {
            id,
            _marker: PhantomData,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hacks_for_code_coverage() {
        // This is optimized away due to being a Copy type.
        let _ = ValId::<()>::new(0).clone();
    }
}
