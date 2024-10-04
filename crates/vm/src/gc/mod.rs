use bumpalo::Bump;
use compact_str::CompactString;
use keep_reachable_set::KeepReachableSet;
#[allow(unused_imports)]
use log::*;
use symbol_interner::SymbolInterner;

use crate::{
    gc::object_store::{Color, ObjectStore},
    val::{custom::CustomVal, ByteCode, ListVal, StructVal, Symbol, UnsafeVal, ValId},
};
mod keep_reachable_set;
mod object_store;
mod symbol_interner;

type BumpVec<'a, T> = bumpalo::collections::Vec<'a, T>;

/// ValStore manages the lifetime of Val objects.
#[derive(Debug)]
pub struct MemoryManager {
    vm_id: u16,
    interned_symbols: SymbolInterner,
    strings: ObjectStore<CompactString>,
    mutable_boxes: ObjectStore<UnsafeVal>,
    lists: ObjectStore<ListVal>,
    structs: ObjectStore<StructVal>,
    bytecodes: ObjectStore<ByteCode>,
    customs: ObjectStore<CustomVal>,
    keep_reachable: KeepReachableSet,
    reachable_color: Color,
}

impl MemoryManager {
    /// Create a new memory manager for the [Vm] with the given id.
    pub fn new(vm_id: u16) -> Self {
        MemoryManager {
            vm_id,
            interned_symbols: SymbolInterner::new(vm_id),
            strings: ObjectStore::default(),
            mutable_boxes: ObjectStore::default(),
            lists: ObjectStore::default(),
            structs: ObjectStore::default(),
            bytecodes: ObjectStore::default(),
            customs: ObjectStore::default(),
            keep_reachable: KeepReachableSet::default(),
            reachable_color: Color::default(),
        }
    }

    /// Get an interned string's value.
    pub fn symbol_to_str(&self, s: Symbol) -> Option<&str> {
        self.interned_symbols.symbol_to_str(s)
    }

    /// Get an interned string.
    pub fn get_symbol(&self, s: &str) -> Option<Symbol> {
        self.interned_symbols.get_symbol(s)
    }

    /// Get or create an interned string.
    pub fn get_or_create_symbol(&mut self, s: &str) -> Symbol {
        self.interned_symbols.get_or_create_symbol(self.vm_id, s)
    }

    /// Run the garbage collector. All known values must be in `values`.
    pub fn run_gc(&mut self, arena: &Bump, populate_vals: impl Iterator<Item = UnsafeVal>) {
        self.run_gc_mark(arena, populate_vals);
        self.run_gc_sweep();
        self.reachable_color = self.reachable_color.other();
    }

    /// Run the GC mark phase.
    fn run_gc_mark(&mut self, arena: &Bump, values: impl Iterator<Item = UnsafeVal>) {
        let mut root_set = self.init_root_set(arena, values);
        let mut child_set = BumpVec::new_in(arena);
        while !root_set.is_empty() {
            for val in root_set.drain(..) {
                self.gc_mark_one(val, &mut child_set);
            }
            std::mem::swap(&mut root_set, &mut child_set);
        }
    }

    /// Initialize the GC mark phase. This takes `values` and enqueues them for marking in
    /// `temp_data.current_queue`.
    fn init_root_set<'a>(
        &self,
        arena: &'a Bump,
        values: impl Iterator<Item = UnsafeVal>,
    ) -> BumpVec<'a, UnsafeVal> {
        let mut root_set = BumpVec::new_in(arena);
        root_set.extend(values);
        root_set.extend(self.keep_reachable.iter());
        root_set
    }

    fn gc_mark_one(&mut self, val: UnsafeVal, child_queue: &mut BumpVec<UnsafeVal>) {
        let mut add_child = |v| {
            if is_garbage_collected(v) {
                child_queue.push(v);
            }
        };
        match val {
            UnsafeVal::String(id) => {
                self.strings.set_color(id, self.reachable_color);
            }
            UnsafeVal::MutableBox(id) => {
                if let Some(unboxed) = self.mutable_boxes.set_color(id, self.reachable_color) {
                    add_child(*unboxed);
                }
            }
            UnsafeVal::List(id) => {
                if let Some(list) = self.lists.set_color(id, self.reachable_color) {
                    for child_val in list.iter() {
                        add_child(*child_val);
                    }
                }
            }
            UnsafeVal::Struct(id) => {
                if let Some(strct) = self.structs.set_color(id, self.reachable_color) {
                    for child_val in strct.values() {
                        add_child(child_val);
                    }
                }
            }
            UnsafeVal::ByteCodeFunction(id) => {
                if let Some(bc) = self.bytecodes.set_color(id, self.reachable_color) {
                    for child_val in bc.values() {
                        add_child(child_val);
                    }
                }
            }
            UnsafeVal::Custom(id) => {
                self.customs.set_color(id, self.reachable_color);
            }

            v => debug_assert!(!is_garbage_collected(v)),
        }
    }

    fn run_gc_sweep(&mut self) {
        let unreachable_color = self.reachable_color.other();
        self.mutable_boxes.remove_all_with_color(unreachable_color);
        self.lists.remove_all_with_color(unreachable_color);
        self.structs.remove_all_with_color(unreachable_color);
        self.bytecodes.remove_all_with_color(unreachable_color);
        self.customs.remove_all_with_color(unreachable_color);
    }

    /// Marks `value` as reachable so that it doesn't get garbage collected.
    pub fn keep_reachable(&mut self, value: UnsafeVal) {
        self.keep_reachable.insert(value);
    }

    /// Removes the `rechable` marking set by `keep_reachable` so that the value may get garbage
    /// collected.
    pub fn allow_unreachable(&mut self, value: UnsafeVal) {
        self.keep_reachable.remove(value);
    }

    /// Get a string by its id.
    pub fn get_str(&self, id: ValId<CompactString>) -> &str {
        self.strings
            .get(self.vm_id, id)
            .map(CompactString::as_str)
            .unwrap_or("")
    }

    /// Insert a string and get its id.
    pub fn insert_string(&mut self, s: CompactString) -> ValId<CompactString> {
        self.strings.insert(self.vm_id, s, self.reachable_color)
    }

    /// Get a string by its id.
    pub fn get_mutable_box(&self, id: ValId<UnsafeVal>) -> &UnsafeVal {
        let res = self.mutable_boxes.get(self.vm_id, id);
        debug_assert!(res.is_some());
        res.unwrap()
    }

    /// Set the mutable box value and return the previous value.
    pub fn set_mutable_box(&mut self, id: ValId<UnsafeVal>, v: UnsafeVal) -> UnsafeVal {
        let slot = self.mutable_boxes.get_mut(self.vm_id, id).unwrap();
        let old = *slot;
        *slot = v;
        old
    }

    /// Insert a string and get its id.
    pub fn insert_mutable_box(&mut self, v: UnsafeVal) -> ValId<UnsafeVal> {
        self.mutable_boxes
            .insert(self.vm_id, v, self.reachable_color.other())
    }

    pub const EMPTY_LIST: &ListVal = &ListVal::new();

    /// Get a list by its id.
    pub fn get_list(&self, id: ValId<ListVal>) -> &ListVal {
        let res = self.lists.get(self.vm_id, id);
        debug_assert!(res.is_some(), "{id:?} not found.");
        res.unwrap_or(Self::EMPTY_LIST)
    }

    /// Insert a list and get its id.
    pub fn insert_list(&mut self, list: ListVal) -> ValId<ListVal> {
        // We mark as unreachable to recurse through `list`'s elements during the next GC mark
        // phase.
        self.lists
            .insert(self.vm_id, list, self.reachable_color.other())
    }

    /// Get a struct by its id.
    pub fn get_struct(&self, id: ValId<StructVal>) -> &StructVal {
        let res = self.structs.get(self.vm_id, id);
        assert!(res.is_some(), "{id:?} not found.");
        res.unwrap()
    }

    /// Get a struct by its id.
    pub fn get_struct_mut(&mut self, id: ValId<StructVal>) -> &mut StructVal {
        let res = self.structs.get_mut(self.vm_id, id);
        assert!(res.is_some(), "{id:?} not found.");
        res.unwrap()
    }

    /// Insert a struct and get its id.
    pub fn insert_struct(&mut self, strct: StructVal) -> ValId<StructVal> {
        // We mark as unreachable to recurse through `list`'s elements during the next GC mark
        // phase.
        self.structs
            .insert(self.vm_id, strct, self.reachable_color.other())
    }

    /// Get a bytecode by its id.
    pub fn get_bytecode(&self, id: ValId<ByteCode>) -> Option<&ByteCode> {
        let res = self.bytecodes.get(self.vm_id, id);
        if res.is_none() {
            error!("{id:?} not found");
        }
        res
    }

    /// Get bytecode id for any bytecode that is equal to `bytecode`. If it does not exist, then it
    /// is inserted into the map.
    ///
    /// Warning: This may be very slow.
    #[cfg(test)]
    pub fn get_or_insert_bytecode_slow(&mut self, bytecode: ByteCode) -> ValId<ByteCode> {
        for (id, val) in self.bytecodes.iter(self.vm_id) {
            if val == &bytecode {
                return id;
            }
        }
        // Untested OK: This typically triggers when tests fail.
        self.insert_bytecode(bytecode)
    }

    /// Insert bytecode into the store and return its id.
    pub fn insert_bytecode(&mut self, bytecode: ByteCode) -> ValId<ByteCode> {
        // We mark as unreachable to recurse through `list`'s elements during the next GC mark
        // phase.
        self.bytecodes
            .insert(self.vm_id, bytecode, self.reachable_color.other())
    }

    /// Get a custom value by its id.
    pub fn get_custom(&self, id: ValId<CustomVal>) -> &CustomVal {
        let c = self.customs.get(self.vm_id, id);
        debug_assert!(c.is_some(), "{id:?} not found");
        c.unwrap()
    }

    /// Insert a custom value and get its id.
    pub fn insert_custom(&mut self, custom: CustomVal) -> ValId<CustomVal> {
        // We mark as unreachable to recurse through `list`'s elements during the next GC mark
        // phase.
        self.customs
            .insert(self.vm_id, custom, self.reachable_color.other())
    }
}

/// Returns `true` if `v` is managed by the garbage collector.
pub fn is_garbage_collected(v: UnsafeVal) -> bool {
    match v {
        UnsafeVal::Void => false,
        UnsafeVal::Bool(_) => false,
        UnsafeVal::Int(_) => false,
        UnsafeVal::Float(_) => false,
        UnsafeVal::Symbol(_) => false,
        UnsafeVal::String(_) => true,
        UnsafeVal::MutableBox(_) => true,
        UnsafeVal::List(_) => true,
        UnsafeVal::Struct(_) => true,
        UnsafeVal::ByteCodeFunction(_) => true,
        UnsafeVal::NativeFunction(_) => false,
        UnsafeVal::Custom(_) => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hacks_for_code_coverage() {
        // This is optimized away due to being a Copy type.
        let _ = ValId {
            vm_id: 0,
            obj_id: 0,
            idx: 0,
            _marker: std::marker::PhantomData::<()>,
        }
        .clone();
    }
}
