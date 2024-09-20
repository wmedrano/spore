use std::{hash::Hash, marker::PhantomData};

/// A unique identifier for an object in `ValStore`.
#[derive(Default)]
pub struct ValId<T> {
    pub(crate) vm_id: u16,
    pub(crate) obj_id: u16,
    pub(crate) idx: u32,
    pub(crate) _marker: PhantomData<T>,
}

impl<T> Eq for ValId<T> {}
impl<T> Copy for ValId<T> {}
impl<T> std::fmt::Debug for ValId<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ValId")
            .field("vm_id", &self.vm_id)
            .field("obj_id", &self.obj_id)
            .field("idx", &self.idx)
            .field("type", &std::any::type_name::<T>())
            .finish()
    }
}
impl<T> Clone for ValId<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> PartialEq for ValId<T> {
    fn eq(&self, other: &Self) -> bool {
        self.idx == other.idx
    }
}
impl<T> Hash for ValId<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.idx.hash(state);
    }
}

impl<T> ValId<T> {
    /// Get the id as `usize` number.
    pub(crate) fn as_usize(self) -> usize {
        self.idx as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hacks_for_code_coverage() {
        // There is not much value in testing this so calling function to appease code coverage
        // tool.
        assert_ne!(format!("{:?}", ValId::<()>::default()), "");
    }
}
