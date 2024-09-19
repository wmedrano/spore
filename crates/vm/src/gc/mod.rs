use std::sync::Arc;

use compact_str::CompactString;

use crate::{
    gc::object_store::{Color, ObjectStore},
    val::{custom::CustomVal, internal::InternalValImpl, ByteCode, InternalVal, ListVal, ValId},
};

mod object_store;

/// ValStore manages the lifetime of Val objects.
#[derive(Debug, Default)]
pub struct MemoryManager {
    strings: ObjectStore<CompactString>,
    mutable_boxes: ObjectStore<InternalVal>,
    lists: ObjectStore<ListVal>,
    bytecodes: ObjectStore<Arc<ByteCode>>,
    customs: ObjectStore<CustomVal>,
    reachable_color: Color,
    stats: GcStats,
    // Data used for GC mark phase.
    temp_mark_data: TempMarkData,
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
    /// The total number of mutable boxes freed.
    mutable_boxes_freed: usize,
    /// The total number of mutable boxes allocated.
    mutable_boxes_allocated: usize,
    /// The total number of lists allocated.
    lists_allocated: usize,
    /// The total number of lists freed.
    lists_freed: usize,
    /// The total number of bytecodes allocated.
    bytecodes_allocated: usize,
    /// The total number of bytecodes freed.
    bytecodes_freed: usize,
    /// The total number of custom values allocated.
    customs_allocated: usize,
    /// The total number of custom values freed.
    customs_freed: usize,
}

#[derive(Clone, Debug, Default)]
struct TempMarkData {
    current_queue: Vec<InternalVal>,
    next_queue: Vec<InternalVal>,
}

impl MemoryManager {
    /// Returns the garage collection stats.
    ///
    /// The function is mutable as it updates some metadata components before returning the stats.
    pub fn stats(&mut self) -> &GcStats {
        let mark_queue_size = std::mem::size_of::<InternalVal>()
            * (self.temp_mark_data.current_queue.capacity()
                + self.temp_mark_data.next_queue.capacity());
        self.stats.gc_metadata_size = mark_queue_size
            + self.strings.metadata_size()
            + self.lists.metadata_size()
            + self.bytecodes.metadata_size()
            + self.customs.metadata_size();
        &self.stats
    }

    /// Run the garbage collector. All known values must be in `values`.
    pub fn run_gc(&mut self, populate_vals: impl Fn(&mut Vec<InternalVal>)) {
        self.stats.gc_invocations += 1;
        let mut temp_data = std::mem::take(&mut self.temp_mark_data);
        self.run_gc_mark(&mut temp_data, populate_vals);
        self.temp_mark_data = temp_data;
        self.run_gc_sweep();
        self.reachable_color = self.reachable_color.other();
    }

    /// Run the GC mark phase.
    fn run_gc_mark(
        &mut self,
        temp_data: &mut TempMarkData,
        values: impl Fn(&mut Vec<InternalVal>),
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
    fn init_gc_mark(&self, temp_data: &mut TempMarkData, values: impl Fn(&mut Vec<InternalVal>)) {
        temp_data.clear_retaining_capacity();
        values(&mut temp_data.current_queue);
        for (id, _) in self.strings.iter_keep_reachable() {
            temp_data.current_queue.push(id.into())
        }
        for (id, _) in self.mutable_boxes.iter_keep_reachable() {
            temp_data.current_queue.push(id.into());
        }
        for (id, _) in self.lists.iter_keep_reachable() {
            temp_data.current_queue.push(id.into());
        }
        for (id, _) in self.bytecodes.iter_keep_reachable() {
            temp_data.current_queue.push(id.into());
        }
        for (id, _) in self.customs.iter_keep_reachable() {
            temp_data.current_queue.push(id.into());
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
                self.strings.set_color(id, self.reachable_color);
            }
            InternalValImpl::MutableBox(id) => {
                if let Some(unboxed) = self.mutable_boxes.set_color(id, self.reachable_color) {
                    add_child(*unboxed);
                }
            }
            InternalValImpl::List(id) => {
                if let Some(list) = self.lists.set_color(id, self.reachable_color) {
                    for child_val in list.iter() {
                        add_child(*child_val);
                    }
                }
            }
            InternalValImpl::ByteCodeFunction(id) => {
                if let Some(bc) = self.bytecodes.set_color(id, self.reachable_color) {
                    for child_val in bc.values() {
                        add_child(child_val);
                    }
                }
            }
            InternalValImpl::Custom(id) => {
                self.customs.set_color(id, self.reachable_color);
            }
            _ => {}
        }
    }

    fn run_gc_sweep(&mut self) {
        let unreachable_color = self.reachable_color.other();
        self.stats.strings_freed += self.strings.remove_all_with_color(unreachable_color);
        self.stats.mutable_boxes_freed +=
            self.mutable_boxes.remove_all_with_color(unreachable_color);
        self.stats.lists_freed += self.lists.remove_all_with_color(unreachable_color);
        self.stats.bytecodes_freed += self.bytecodes.remove_all_with_color(unreachable_color);
        self.stats.customs_freed += self.customs.remove_all_with_color(unreachable_color);
    }

    /// Marks `value` as reachable so that it doesn't get garbage collected.
    pub fn keep_reachable(&mut self, value: InternalVal) {
        match value.0 {
            InternalValImpl::Void => {}
            InternalValImpl::Bool(_) => {}
            InternalValImpl::Int(_) => {}
            InternalValImpl::Float(_) => {}
            InternalValImpl::String(id) => {
                self.strings.mark_always_reachable(id);
            }
            InternalValImpl::MutableBox(id) => {
                self.mutable_boxes.mark_always_reachable(id);
            }
            InternalValImpl::List(id) => {
                self.lists.mark_always_reachable(id);
            }
            InternalValImpl::ByteCodeFunction(id) => {
                self.bytecodes.mark_always_reachable(id);
            }
            InternalValImpl::NativeFunction(_) => {}
            InternalValImpl::Custom(id) => self.customs.mark_always_reachable(id),
        }
    }

    /// Removes the `rechable` marking set by `keep_reachable` so that the value may get garbage
    /// collected.
    pub fn allow_unreachable(&mut self, value: InternalVal) {
        match value.0 {
            InternalValImpl::Void => {}
            InternalValImpl::Bool(_) => {}
            InternalValImpl::Int(_) => {}
            InternalValImpl::Float(_) => {}
            InternalValImpl::MutableBox(id) => {
                self.mutable_boxes.unmark_always_reachable(id);
            }
            InternalValImpl::String(id) => {
                self.strings.unmark_always_reachable(id);
            }
            InternalValImpl::List(id) => {
                self.lists.unmark_always_reachable(id);
            }
            InternalValImpl::ByteCodeFunction(id) => {
                self.bytecodes.unmark_always_reachable(id);
            }
            InternalValImpl::NativeFunction(_) => {}
            InternalValImpl::Custom(id) => self.customs.unmark_always_reachable(id),
        }
    }

    /// Get a string by its id.
    pub fn get_str(&self, id: ValId<CompactString>) -> &str {
        let res = self.strings.get(id);
        debug_assert!(res.is_some());
        res.map(CompactString::as_str).unwrap_or("")
    }

    /// Insert a string and get its id.
    pub fn insert_string(&mut self, s: CompactString) -> ValId<CompactString> {
        self.stats.strings_allocated += 1;
        self.strings.insert(s, self.reachable_color)
    }

    /// Get a string by its id.
    pub fn get_mutable_box(&self, id: ValId<InternalVal>) -> &InternalVal {
        let res = self.mutable_boxes.get(id);
        debug_assert!(res.is_some());
        res.unwrap()
    }

    /// Set the mutable box value and return the previous value.
    pub fn set_mutable_box(&mut self, id: ValId<InternalVal>, v: InternalVal) -> InternalVal {
        let slot = self.mutable_boxes.get_mut(id).unwrap();
        let old = *slot;
        *slot = v;
        old
    }

    /// Insert a string and get its id.
    pub fn insert_mutable_box(&mut self, v: InternalVal) -> ValId<InternalVal> {
        self.stats.mutable_boxes_allocated += 1;
        self.mutable_boxes.insert(v, self.reachable_color)
    }

    pub const EMPTY_LIST: &ListVal = &ListVal::new();

    /// Get a list by its id.
    pub fn get_list(&self, id: ValId<ListVal>) -> &ListVal {
        let res = self.lists.get(id);
        debug_assert!(res.is_some(), "{id:?} not found.");
        res.unwrap_or(Self::EMPTY_LIST)
    }

    /// Insert a list and get its id.
    pub fn insert_list(&mut self, list: ListVal) -> ValId<ListVal> {
        self.stats.lists_allocated += 1;
        // We mark as unreachable to recurse through `list`'s elements during the next GC mark
        // phase.
        self.lists.insert(list, self.reachable_color.other())
    }

    /// Get a bytecode by its id.
    pub fn get_bytecode(&self, id: ValId<Arc<ByteCode>>) -> &Arc<ByteCode> {
        let res = self.bytecodes.get(id);
        debug_assert!(res.is_some(), "{id:?} not found");
        res.unwrap()
    }

    /// Get bytecode id for any bytecode that is equal to `bytecode`. If it does not exist, then it
    /// is inserted into the map.
    ///
    /// Warning: This may be very slow.
    #[cfg(test)]
    pub fn get_or_insert_bytecode_slow(&mut self, bytecode: ByteCode) -> ValId<Arc<ByteCode>> {
        for (id, val) in self.bytecodes.iter() {
            if val.as_ref() == &bytecode {
                return id;
            }
        }
        // Untested OK: This typically triggers when tests fail.
        self.insert_bytecode(bytecode)
    }

    /// Insert bytecode into the store and return its id.
    pub fn insert_bytecode(&mut self, bytecode: ByteCode) -> ValId<Arc<ByteCode>> {
        self.stats.bytecodes_allocated += 1;
        // We mark as unreachable to recurse through `list`'s elements during the next GC mark
        // phase.
        self.bytecodes
            .insert(bytecode.into(), self.reachable_color.other())
    }

    /// Get a custom value by its id.
    pub fn get_custom(&self, id: ValId<CustomVal>) -> &CustomVal {
        let c = self.customs.get(id);
        debug_assert!(c.is_some(), "{id:?} not found");
        c.unwrap()
    }

    /// Insert a custom value and get its id.
    pub fn insert_custom(&mut self, custom: CustomVal) -> ValId<CustomVal> {
        self.stats.customs_allocated += 1;
        // We mark as unreachable to recurse through `list`'s elements during the next GC mark
        // phase.
        self.customs.insert(custom, self.reachable_color.other())
    }
}

/// Returns `true` if `v` is managed by the garbage collector.
pub fn is_garbage_collected(v: InternalVal) -> bool {
    match v.0 {
        InternalValImpl::Void => false,
        InternalValImpl::Bool(_) => false,
        InternalValImpl::Int(_) => false,
        InternalValImpl::Float(_) => false,
        InternalValImpl::String(_) => true,
        InternalValImpl::MutableBox(_) => true,
        InternalValImpl::List(_) => true,
        InternalValImpl::ByteCodeFunction(_) => true,
        InternalValImpl::NativeFunction(_) => false,
        InternalValImpl::Custom(_) => true,
    }
}

impl TempMarkData {
    fn clear_retaining_capacity(&mut self) {
        self.current_queue.clear();
        self.next_queue.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hacks_for_code_coverage() {
        // This is optimized away due to being a Copy type.
        let _ = ValId::<()>::new(0u32).clone();
    }
}
