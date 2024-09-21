use std::ops::Deref;

#[allow(unused_imports)]
use log::*;

use crate::Vm;

use super::{
    custom::{CustomValError, CustomValMut, CustomValRef},
    CustomType, Val,
};

/// Holds a Value from the [Vm]. Unlike [UnsafeVal], the underlying value is guaranteed to not be
/// garbage collected.
#[derive(Debug)]
pub struct ProtectedVal<'a> {
    pub(crate) vm: &'a mut Vm,
    pub(crate) v: Val<'a>,
    _internal: (),
}

impl<'a> ProtectedVal<'a> {
    /// Create a new `Val` from an [UnsafeVal].
    ///
    /// # Safety
    /// `v` must be a valid from originating (and not garbage collected) from `vm`.
    pub fn new(vm: &'a mut Vm, v: Val<'a>) -> ProtectedVal<'a> {
        vm.objects.keep_reachable(v.inner);
        ProtectedVal {
            vm,
            v,
            _internal: (),
        }
    }

    /// Split the protected val into its VM and `Val`.
    pub fn split(&mut self) -> (&mut Vm, &Val) {
        (self.vm, &self.v)
    }
}

impl<'a> Drop for ProtectedVal<'a> {
    fn drop(&mut self) {
        self.vm.objects.allow_unreachable(self.v.inner);
    }
}

impl<'a> Deref for ProtectedVal<'a> {
    type Target = Val<'a>;

    fn deref(&self) -> &Self::Target {
        &self.v
    }
}

impl<'a> ProtectedVal<'a> {
    /// Get a reference to the underlying [Vm].
    pub fn vm(&self) -> &Vm {
        self.vm
    }

    pub fn map<T>(&mut self, f: impl Fn(&mut Vm, &ProtectedVal<'a>) -> T) -> T {
        let protected_val_ptr: *const ProtectedVal = self;
        // Unsafe OK: Protected val will still be safe from garbage collection as drop has not been
        // called.
        f(self.vm, unsafe { &*protected_val_ptr })
    }

    /// Try to get the string as a value or return its underlying value.
    pub fn try_str(&'a self) -> Result<&'a str, Val> {
        self.v.try_str(self.vm)
    }

    /// value.
    pub fn as_custom<T: CustomType>(&self) -> Result<CustomValRef<T>, CustomValError> {
        self.v.as_custom(self.vm)
    }

    /// Get the [Val] that the mutable box is pointing to or [Err<Val>] if `self` is not a mutable
    /// box.
    pub fn get_mutable_box_ref(&self) -> Result<Val, Val<'a>> {
        self.v.get_mutable_box_ref(self.vm)
    }

    /// Returns the value as a custom type of `T` or [None] if [Self] is not of the given custom
    /// value.
    pub fn as_custom_mut<T: CustomType>(&self) -> Result<CustomValMut<T>, CustomValError> {
        self.v.as_custom_mut(self.vm)
    }
}

impl<'a> std::fmt::Display for ProtectedVal<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.v.formatted(self.vm).fmt(f)
    }
}
