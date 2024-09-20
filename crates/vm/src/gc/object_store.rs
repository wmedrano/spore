use std::marker::PhantomData;

use log::*;

use crate::val::ValId;

#[derive(Clone, Debug)]
pub struct ValWithColor<T> {
    /// The underlying object or `None` if the slot does not contain an object.
    inner: Option<T>,
    /// The id of the underlying object.
    id: u16,
    /// The color of the object. Used for marking data as reachable (to keep) or unreachable (to
    /// garbage collect).
    color: Color,
}

/// A collection of `T` values that may be garbage collected.
#[derive(Clone, Debug)]
pub struct ObjectStore<T> {
    /// Backing storage for all the objects.
    objects: Vec<ValWithColor<T>>,
    /// List of id (indices) for free entries within [objects].
    free_object_idx: Vec<u32>,
}

impl<T> Default for ObjectStore<T> {
    fn default() -> ObjectStore<T> {
        ObjectStore {
            objects: Vec::new(),
            free_object_idx: Vec::new(),
        }
    }
}

impl<T: std::fmt::Debug> ObjectStore<T> {
    /// The size of metadata structures in bytes.
    pub fn metadata_size(&self) -> usize {
        std::mem::size_of::<ValWithColor<T>>() * self.objects.capacity()
            + std::mem::size_of::<ValId<T>>() * self.free_object_idx.capacity()
    }

    /// Returns the object if its color was changed.
    pub fn set_color(&mut self, id: ValId<T>, new_color: Color) -> Option<&T> {
        match self.objects.get_mut(id.as_usize()) {
            Some(ValWithColor {
                inner: Some(inner),
                color,
                ..
            }) if *color != new_color => {
                *color = new_color;
                Some(inner)
            }
            _ => None,
        }
    }

    /// Remove all objects with the given `color`. Note, any objects that activated
    /// [Self::mark_always_reachable] will not be cleaned up unless undone with
    /// [Self::unmark_always_reachable].
    ///
    /// Returns the number of objects that were removed.
    pub fn remove_all_with_color(&mut self, color: Color) -> usize {
        let start_free = self.free_object_idx.len();
        for (idx, obj) in self.objects.iter_mut().enumerate() {
            if obj.inner.is_some() && obj.color == color {
                self.free_object_idx.push(idx as _);
            }
        }
        self.free_object_idx.len() - start_free
    }

    #[cfg(test)]
    pub fn iter(&self, vm_id: u16) -> impl '_ + Iterator<Item = (ValId<T>, &T)> {
        self.objects
            .iter()
            .enumerate()
            .map(move |(idx, obj)| {
                (
                    ValId {
                        vm_id,
                        obj_id: obj.id,
                        idx: idx as _,
                        _marker: PhantomData,
                    },
                    obj,
                )
            })
            .flat_map(|(id, v)| v.inner.as_ref().map(|obj| (id, obj)))
    }

    /// Get a reference to the underlying type or `None` if it does not exist.
    pub fn get(&self, vm_id: u16, id: ValId<T>) -> Option<&T> {
        if vm_id != id.vm_id {
            return None;
        }
        self.objects
            .get(id.as_usize())
            .filter(|obj| {
                if obj.id == id.obj_id {
                    true
                } else {
                    error!(
                        "Wrong object id found, expected {expected:?} but found {actual:?}.",
                        expected = id.obj_id,
                        actual = obj.id
                    );
                    false
                }
            })
            .and_then(|obj| obj.inner.as_ref())
    }

    /// Get a mutable reference to the underlying type or `None` if it does not exist.
    pub fn get_mut(&mut self, vm_id: u16, id: ValId<T>) -> Option<&mut T> {
        if vm_id != id.vm_id {
            return None;
        }
        self.objects
            .get_mut(id.as_usize())
            .filter(|obj| obj.id == id.obj_id)
            .and_then(|obj| obj.inner.as_mut())
    }

    /// Insert object `T` with `color` and return its `id`.
    pub fn insert(&mut self, vm_id: u16, obj: T, color: Color) -> ValId<T> {
        match self.free_object_idx.pop() {
            Some(idx) => {
                let obj_id = self.objects[idx as usize].id.wrapping_add(1);
                self.objects[idx as usize] = ValWithColor {
                    inner: Some(obj),
                    id: obj_id,
                    color,
                };
                ValId {
                    vm_id,
                    obj_id,
                    idx,
                    _marker: PhantomData,
                }
            }
            None => {
                let id = ValId {
                    vm_id,
                    obj_id: 0,
                    idx: self.objects.len() as _,
                    _marker: PhantomData,
                };
                self.objects.push(ValWithColor {
                    inner: Some(obj),
                    id: id.obj_id,
                    color,
                });
                id
            }
        }
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum Color {
    /// A color that may mark if a value is used or unused.
    #[default]
    Red,
    /// A color that may mark if a value is used or unused.
    Blue,
}

impl Color {
    /// Returns the other color.
    pub fn other(self) -> Color {
        match self {
            Color::Red => Color::Blue,
            Color::Blue => Color::Red,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use compact_str::CompactString;

    use crate::val::{custom::CustomVal, ByteCode, UnsafeVal};

    use super::*;

    #[test]
    fn sizes_are_small() {
        assert_eq!(
            std::mem::size_of::<ValWithColor<CompactString>>(),
            8 + std::mem::size_of::<String>()
        );
        assert_eq!(
            std::mem::size_of::<ValWithColor<UnsafeVal>>(),
            8 + std::mem::size_of::<UnsafeVal>()
        );
        assert_eq!(
            std::mem::size_of::<ValWithColor<Arc<ByteCode>>>(),
            8 + std::mem::size_of::<Arc<ByteCode>>(),
        );
        assert_eq!(
            std::mem::size_of::<ValWithColor<CustomVal>>(),
            8 + std::mem::size_of::<CustomVal>()
        );
    }
}
