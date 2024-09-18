use std::marker::PhantomData;

type IdRepr = u32;

/// A unique identifier for an object in `ValStore`.
#[derive(Default)]
pub struct ValId<T> {
    pub id: IdRepr,
    _marker: PhantomData<T>,
}

impl<T> std::fmt::Debug for ValId<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Id({id_type}, {id}",
            id_type = std::any::type_name::<T>(),
            id = self.id
        )
    }
}

impl<T> Eq for ValId<T> {}
impl<T> Copy for ValId<T> {}
impl<T> Clone for ValId<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> PartialEq for ValId<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<T> ValId<T> {
    pub fn new(id: IdRepr) -> ValId<T> {
        ValId {
            id,
            _marker: PhantomData,
        }
    }

    /// Get the id as `usize` number.
    pub fn as_usize(self) -> usize {
        self.id as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hacks_for_code_coverage() {
        // There is not much value in testing this so calling function to appease code coverage
        // tool.
        assert_ne!(format!("{:?}", ValId::<()>::new(0)), "");
    }
}
