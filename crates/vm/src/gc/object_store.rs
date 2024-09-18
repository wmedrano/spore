use crate::val::ValId;

#[derive(Clone, Debug)]
pub struct ValWithColor<T> {
    /// The underlying object or `None` if the slot does not contain an object.
    inner: Option<T>,
    /// The color of the object. Used for marking data as reachable (to keep) or unreachable (to
    /// garbage collect).
    color: Color,
    /// The number of manually labeled reachable references.
    keep_reachable_count: u32,
}

#[derive(Clone, Debug)]
pub struct ObjectStore<T> {
    objects: Vec<ValWithColor<T>>,
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

impl<T> ObjectStore<T> {
    /// Mark `id` as always reachable.
    pub fn keep_reachable(&mut self, id: ValId<T>) {
        if let Some(obj) = self.objects.get_mut(id.as_usize()) {
            obj.keep_reachable_count += 1;
        }
    }

    /// Allow `id` to be labeled as `unreachable`.
    pub fn allow_unreachable(&mut self, id: ValId<T>) {
        if let Some(obj) = self.objects.get_mut(id.as_usize()) {
            obj.keep_reachable_count -= 1;
        }
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

    pub fn remove_all_with_color(&mut self, color: Color) {
        for (idx, obj) in self.objects.iter_mut().enumerate() {
            if obj.inner.is_some() && obj.color == color && obj.keep_reachable_count == 0 {
                obj.keep_reachable_count = 0;
                obj.inner.take();
                self.free_object_ids.push(ValId::new(idx as u32));
            }
        }
    }

    /// Iterate over all objects that are marked as keep reachable.
    pub fn iter_keep_reachable(&self) -> impl '_ + Iterator<Item = (ValId<T>, &T)> {
        self.iter_impl().flat_map(|(id, obj)| {
            match (obj.keep_reachable_count, obj.inner.as_ref()) {
                (0, _) | (_, None) => None,
                (_, Some(obj)) => Some((id, obj)),
            }
        })
    }

    #[cfg(test)]
    pub fn iter(&self) -> impl '_ + Iterator<Item = (ValId<T>, &T)> {
        self.iter_impl()
            .flat_map(|(id, v)| v.inner.as_ref().map(|obj| (id, obj)))
    }

    fn iter_impl(&self) -> impl '_ + Iterator<Item = (ValId<T>, &'_ ValWithColor<T>)> {
        self.objects
            .iter()
            .enumerate()
            .map(|(idx, obj)| (ValId::new(idx as u32), obj))
    }

    pub fn get(&self, id: ValId<T>) -> Option<&T> {
        self.objects
            .get(id.as_usize())
            .and_then(|obj| obj.inner.as_ref())
    }

    pub fn insert(&mut self, obj: T, color: Color) -> ValId<T> {
        let v = ValWithColor {
            inner: Some(obj),
            color,
            keep_reachable_count: 0,
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
    use std::{any::Any, sync::Arc};

    use smol_str::SmolStr;

    use crate::val::ByteCode;

    use super::*;

    #[test]
    fn sizes_are_small() {
        assert_eq!(
            std::mem::size_of::<ValWithColor<Box<dyn Any>>>(),
            8 + std::mem::size_of::<Box<dyn Any>>()
        );
        assert_eq!(std::mem::size_of::<SmolStr>(), 24);
        assert_eq!(
            std::mem::size_of::<ValWithColor<SmolStr>>(),
            8 + std::mem::size_of::<SmolStr>()
        );
        assert_eq!(
            std::mem::size_of::<ValWithColor<Arc<ByteCode>>>(),
            8 + std::mem::size_of::<Arc<ByteCode>>(),
        );
    }
}
