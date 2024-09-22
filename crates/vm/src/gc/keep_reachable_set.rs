use std::{
    collections::{hash_map::Entry, HashMap},
    hash::Hash,
    sync::Arc,
};

use compact_str::CompactString;
use log::*;

use crate::val::{custom::CustomVal, ByteCode, ListVal, UnsafeVal, ValId};

use super::is_garbage_collected;

type ReferenceCounter = usize;

/// A set that keeps tracks of garbage collected values to keep reachable. Keeping reachable
/// prevents the garbage collector from cleaning up its values.
#[derive(Debug, Default)]
pub struct KeepReachableSet {
    strings: HashMap<ValId<CompactString>, ReferenceCounter>,
    mutable_boxes: HashMap<ValId<UnsafeVal>, ReferenceCounter>,
    lists: HashMap<ValId<ListVal>, ReferenceCounter>,
    bytecodes: HashMap<ValId<Arc<ByteCode>>, ReferenceCounter>,
    customs: HashMap<ValId<CustomVal>, ReferenceCounter>,
}

impl KeepReachableSet {
    /// Get the size of the keep reachable set in bytes.
    pub fn bytes_size(&self) -> usize {
        self.strings.byte_size()
            + self.mutable_boxes.byte_size()
            + self.lists.byte_size()
            + self.bytecodes.byte_size()
            + self.customs.byte_size()
    }

    /// Iterate over all values that should be kept reachable.
    pub fn iter(&self) -> impl '_ + Iterator<Item = UnsafeVal> {
        self.strings
            .keys()
            .copied()
            .map(Into::into)
            .chain(self.mutable_boxes.keys().copied().map(Into::into))
            .chain(self.lists.keys().copied().map(Into::into))
            .chain(self.bytecodes.keys().copied().map(Into::into))
            .chain(self.customs.keys().copied().map(Into::into))
    }

    /// Insert a value into the set of reachable values.
    pub fn insert(&mut self, val: UnsafeVal) {
        match val {
            UnsafeVal::String(x) => self.strings.increment(x),
            UnsafeVal::MutableBox(x) => self.mutable_boxes.increment(x),
            UnsafeVal::List(x) => self.lists.increment(x),
            UnsafeVal::ByteCodeFunction(x) => self.bytecodes.increment(x),
            UnsafeVal::Custom(x) => self.customs.increment(x),
            v => assert!(!is_garbage_collected(v)),
        }
    }

    /// Remove a value from the set to keep reachable.
    pub fn remove(&mut self, val: UnsafeVal) {
        match val {
            UnsafeVal::String(x) => self.strings.decrement(x),
            UnsafeVal::MutableBox(x) => self.mutable_boxes.decrement(x),
            UnsafeVal::List(x) => self.lists.decrement(x),
            UnsafeVal::ByteCodeFunction(x) => self.bytecodes.decrement(x),
            UnsafeVal::Custom(x) => self.customs.decrement(x),
            v => assert!(!is_garbage_collected(v)),
        }
    }
}

/// A private trait with some helper methods around reachable counters.
trait ReachableStoreSealed {
    type K: Copy + std::fmt::Debug + Hash + Eq;
    fn as_hashmap(&self) -> &HashMap<Self::K, ReferenceCounter>;
    fn as_mut_hashmap(&mut self) -> &mut HashMap<Self::K, ReferenceCounter>;

    fn byte_size(&self) -> usize {
        self.as_hashmap().capacity() * std::mem::size_of::<(Self::K, ReferenceCounter)>()
    }

    fn increment(&mut self, k: Self::K) {
        match self.as_mut_hashmap().entry(k) {
            Entry::Occupied(entry) => {
                *entry.into_mut() += 1;
            }
            Entry::Vacant(entry) => {
                entry.insert(1);
            }
        };
    }
    fn decrement(&mut self, k: Self::K) {
        match self.as_mut_hashmap().entry(k) {
            Entry::Occupied(mut entry) => {
                match entry.get().saturating_sub(1) {
                    0 => entry.remove(),
                    x => entry.insert(x),
                };
            }
            Entry::Vacant(_) => {
                warn!("Tried to remove non-existant value {k:?} from keep reachable set. The object reference likely outlived its lifetime. Although not a not considered unsafe, this is likely a memory leak.");
            }
        };
    }
}

impl<T: std::fmt::Debug> ReachableStoreSealed for HashMap<ValId<T>, ReferenceCounter> {
    type K = ValId<T>;
    fn as_hashmap(&self) -> &HashMap<Self::K, ReferenceCounter> {
        self
    }

    fn as_mut_hashmap(&mut self) -> &mut HashMap<Self::K, ReferenceCounter> {
        self
    }
}
