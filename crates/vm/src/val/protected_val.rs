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
    pub fn map<T>(&mut self, f: impl Fn(&mut Vm, &ProtectedVal<'a>) -> T) -> T {
        let protected_val_ptr: *const ProtectedVal = self;
        // Unsafe OK: Protected val will still be safe from garbage collection as drop has not been
        // called.
        f(self.vm, unsafe { &*protected_val_ptr })
    }

    /// Get a reference to the underlying VM.
    pub fn vm_mut(&mut self) -> &mut Vm {
        self.vm
    }

    pub fn as_str(&'a self) -> Option<&'a str> {
        self.v.as_str(&self.vm)
    }

    /// Returns `true` if a custom value is held.
    pub fn is_custom(&self) -> bool {
        self.v.is_custom()
    }

    /// Returns the value as a custom type of `T` or [None] if [Self] is not of the given custom
    /// value.
    pub fn as_custom<T: CustomType>(&self) -> Result<CustomValRef<T>, CustomValError> {
        self.v.as_custom(self.vm)
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
