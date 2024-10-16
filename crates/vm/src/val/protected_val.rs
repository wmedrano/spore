use std::ops::Deref;

#[allow(unused_imports)]
use log::*;

use crate::Vm;

use super::{
    custom::{CustomValError, CustomValMut, CustomValRef},
    CustomType, Val,
};

/// Holds a value from the [Vm] that is guaranteed to not be garbage collected.
///
/// The underlying value is protected from garbage collection until `ProtectedVal` is dropped.
pub struct ProtectedVal<'a> {
    pub(crate) vm: &'a mut Vm,
    pub(crate) val: Val<'a>,
}

impl<'a> std::fmt::Debug for ProtectedVal<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProtectedVal")
            .field("val", &self.val.inner)
            .finish_non_exhaustive()
    }
}

impl<'a> ProtectedVal<'a> {
    /// Create a new `Val` from an [crate::val::UnsafeVal].
    ///
    /// # Safety
    /// `v` must be a valid from originating (and not garbage collected) from `vm`.
    pub fn new(vm: &'a mut Vm, v: Val<'a>) -> ProtectedVal<'a> {
        vm.objects.keep_reachable(v.inner);
        ProtectedVal { vm, val: v }
    }

    /// Split the protected val into its [Vm] and [Val].
    ///
    /// Despite the split, the returned `Val` will still be safe from garbage collection.
    pub fn split(&mut self) -> (&mut Vm, &Val) {
        (self.vm, &self.val)
    }

    /// Run function `f` on the [Vm] and [Val] and return the result. This is a convenient way of
    /// gaining mutable reference to the underlying vm.
    pub fn with<T>(&mut self, f: impl Fn(&mut Vm, &Val) -> T) -> T {
        let (vm, val) = self.split();
        f(vm, val)
    }
}

impl<'a> Drop for ProtectedVal<'a> {
    fn drop(&mut self) {
        self.vm.objects.allow_unreachable(self.val.inner);
    }
}

impl<'a> Deref for ProtectedVal<'a> {
    type Target = Val<'a>;

    fn deref(&self) -> &Self::Target {
        &self.val
    }
}

impl<'a> ProtectedVal<'a> {
    /// Get a reference to the underlying [Vm].
    pub fn vm(&self) -> &Vm {
        self.vm
    }

    /// Maps an `Option<T>` to `Option<U>` by applying a function to a contained value (if `Some`)
    /// or returns `None` (if `None`).
    pub fn map<T>(&mut self, f: impl Fn(&mut Vm, &ProtectedVal<'a>) -> T) -> T {
        let protected_val_ptr: *const ProtectedVal = self;
        // Unsafe OK: Protected val will still be safe from garbage collection as drop has not been
        // called.
        f(self.vm, unsafe { &*protected_val_ptr })
    }

    /// Try to get the string as a value or return its underlying value.
    pub fn try_str(&'a self) -> Result<&'a str, Val> {
        self.val.try_str(self.vm)
    }

    /// Try to get the custom value of type `T` or return its underlying value if `self` is not of
    /// type `T`.
    pub fn try_custom<T: CustomType>(&self) -> Result<CustomValRef<T>, CustomValError> {
        self.val.try_custom(self.vm)
    }

    /// Returns the value as a custom type of `T` or [Err] if [Self] is not of the given custom
    /// value.
    pub fn try_custom_mut<T: CustomType>(&self) -> Result<CustomValMut<T>, CustomValError> {
        self.val.try_custom_mut(self.vm)
    }

    /// Get the [Val] that the mutable box is pointing to or `Err<Val>` if `self` is not a mutable
    /// box.
    pub fn get_mutable_box_ref(&self) -> Result<Val, Val<'a>> {
        self.val.try_mutable_box_ref(self.vm)
    }
}

impl<'a> std::fmt::Display for ProtectedVal<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.val.format_quoted(self.vm).fmt(f)
    }
}
