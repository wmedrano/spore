use bumpalo::Bump;
use compact_str::CompactString;
use keep_reachable_set::KeepReachableSet;
#[allow(unused_imports)]
use log::*;

use crate::{
    gc::object_store::{Color, ObjectStore},
    val::{custom::CustomVal, ByteCode, ListVal, StructVal, UnsafeVal, ValId},
};

mod keep_reachable_set;
mod object_store;

type BumpVec<'a, T> = bumpalo::collections::Vec<'a, T>;

/// ValStore manages the lifetime of Val objects.
#[derive(Debug)]
pub struct MemoryManager {
    vm_id: u16,
    strings: ObjectStore<CompactString>,
    mutable_boxes: ObjectStore<UnsafeVal>,
    lists: ObjectStore<ListVal>,
    structs: ObjectStore<StructVal>,
    bytecodes: ObjectStore<ByteCode>,
    customs: ObjectStore<CustomVal>,
    keep_reachable: KeepReachableSet,
    reachable_color: Color,
    stats: GcStats,
}

/// Contains garbage collection stats for a [MemoryManager].
#[derive(Clone, Debug, Default)]
pub struct GcStats {
    /// The number of times GC was invoked.
    gc_invocations: usize,
    /// The size of GC metadata structures in bytes.
    gc_metadata_size: usize,
    /// The total number of strings allocated.
    strings_allocated: usize,
    /// The total number of strings freed.
    strings_freed: usize,
    /// The total number of mutable boxes allocated.
    mutable_boxes_allocated: usize,
    /// The total number of mutable boxes freed.
    mutable_boxes_freed: usize,
    /// The total number of lists allocated.
    lists_allocated: usize,
    /// The total number of lists freed.
    lists_freed: usize,
    /// The total number of structs allocated.
    structs_allocated: usize,
    /// The total number of structs freed.
    structs_freed: usize,
    /// The total number of bytecodes allocated.
    bytecodes_allocated: usize,
    /// The total number of bytecodes freed.
    bytecodes_freed: usize,
    /// The total number of custom values allocated.
    customs_allocated: usize,
    /// The total number of custom values freed.
    customs_freed: usize,
}

impl MemoryManager {
    /// Create a new memory manager for the [Vm] with the given id.
    pub fn new(vm_id: u16) -> Self {
        MemoryManager {
            vm_id,
            strings: ObjectStore::default(),
            mutable_boxes: ObjectStore::default(),
            lists: ObjectStore::default(),
            structs: ObjectStore::default(),
            bytecodes: ObjectStore::default(),
            customs: ObjectStore::default(),
            keep_reachable: KeepReachableSet::default(),
            reachable_color: Color::default(),
            stats: GcStats::default(),
        }
    }
    /// Returns the garage collection stats.
    ///
    /// The function is mutable as it updates some metadata components before returning the stats.
    pub fn stats(&mut self) -> &GcStats {
        self.stats.gc_metadata_size = self.strings.metadata_size()
            + self.lists.metadata_size()
            + self.bytecodes.metadata_size()
            + self.customs.metadata_size()
            + self.keep_reachable.bytes_size();
        &self.stats
    }

    /// Run the garbage collector. All known values must be in `values`.
    pub fn run_gc(&mut self, arena: &Bump, populate_vals: impl Iterator<Item = UnsafeVal>) {
        self.stats.gc_invocations += 1;
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
                        add_child(*child_val);
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
        self.stats.strings_freed += self.strings.remove_all_with_color(unreachable_color);
        self.stats.mutable_boxes_freed +=
            self.mutable_boxes.remove_all_with_color(unreachable_color);
        self.stats.lists_freed += self.lists.remove_all_with_color(unreachable_color);
        self.stats.structs_freed += self.structs.remove_all_with_color(unreachable_color);
        self.stats.bytecodes_freed += self.bytecodes.remove_all_with_color(unreachable_color);
        self.stats.customs_freed += self.customs.remove_all_with_color(unreachable_color);
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
        self.stats.strings_allocated += 1;
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
        self.stats.mutable_boxes_allocated += 1;
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
        self.stats.lists_allocated += 1;
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

    /// Insert a struct and get its id.
    pub fn insert_struct(&mut self, strct: StructVal) -> ValId<StructVal> {
        self.stats.structs_allocated += 1;
        // We mark as unreachable to recurse through `list`'s elements during the next GC mark
        // phase.
        self.structs
            .insert(self.vm_id, strct, self.reachable_color.other())
    }

    /// Get a bytecode by its id.
    pub fn get_bytecode(&self, id: ValId<ByteCode>) -> &ByteCode {
        let res = self.bytecodes.get(self.vm_id, id);
        debug_assert!(res.is_some(), "{id:?} not found");
        res.unwrap()
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
        self.stats.bytecodes_allocated += 1;
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
        self.stats.customs_allocated += 1;
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
