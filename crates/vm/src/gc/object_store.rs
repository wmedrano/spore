use crate::val::ValId;

#[derive(Clone, Debug)]
pub struct ValWithColor<T> {
    /// The underlying object or `None` if the slot does not contain an object.
    inner: Option<T>,
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
    free_object_ids: Vec<ValId<T>>,
}

impl<T> Default for ObjectStore<T> {
    fn default() -> ObjectStore<T> {
        ObjectStore {
            objects: Vec::new(),
            free_object_ids: Vec::new(),
        }
    }
}

impl<T: std::fmt::Debug> ObjectStore<T> {
    /// The size of metadata structures in bytes.
    pub fn metadata_size(&self) -> usize {
        std::mem::size_of::<ValWithColor<T>>() * self.objects.capacity()
            + std::mem::size_of::<ValId<T>>() * self.free_object_ids.capacity()
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
        let start_free = self.free_object_ids.len();
        for (idx, obj) in self.objects.iter_mut().enumerate() {
            if obj.inner.is_some() && obj.color == color {
                self.free_object_ids.push(ValId::new(idx as u32));
            }
        }
        self.free_object_ids.len() - start_free
    }

    #[cfg(test)]
    pub fn iter(&self) -> impl '_ + Iterator<Item = (ValId<T>, &T)> {
        self.objects
            .iter()
            .enumerate()
            .map(|(idx, obj)| (ValId::new(idx as u32), obj))
            .flat_map(|(id, v)| v.inner.as_ref().map(|obj| (id, obj)))
    }

    /// Get a reference to the underlying type or `None` if it does not exist.
    pub fn get(&self, id: ValId<T>) -> Option<&T> {
        self.objects
            .get(id.as_usize())
            .and_then(|obj| obj.inner.as_ref())
    }

    /// Get a mutable reference to the underlying type or `None` if it does not exist.
    pub fn get_mut(&mut self, id: ValId<T>) -> Option<&mut T> {
        self.objects
            .get_mut(id.as_usize())
            .and_then(|obj| obj.inner.as_mut())
    }

    /// Insert object `T` with `color` and return its `id`.
    pub fn insert(&mut self, obj: T, color: Color) -> ValId<T> {
        let v = ValWithColor {
            inner: Some(obj),
            color,
        };
        match self.free_object_ids.pop() {
            Some(id) => {
                self.objects[id.as_usize()] = v;
                id
            }
            None => {
                let id = ValId::new(self.objects.len() as u32);
                self.objects.push(v);
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
