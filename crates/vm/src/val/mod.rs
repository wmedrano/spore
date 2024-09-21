//! Contains representation of values within the Spore machine. Each type has differing lifetime and
//! safety guarantees.
mod bytecode;
pub(crate) mod custom;
mod formatter;
mod id;
mod native_function;
mod protected_val;
mod unsafe_val;

use std::marker::PhantomData;

pub use bytecode::{ByteCode, Instruction};
pub use custom::CustomType;
use custom::{CustomValError, CustomValMut, CustomValRef};
pub use formatter::ValFormatter;
pub use id::ValId;
pub use native_function::{NativeFunction, NativeFunctionContext, ValBuilder};
pub use protected_val::ProtectedVal;
pub use unsafe_val::UnsafeVal;

use crate::Vm;

/// A container for a list.
pub type ListVal = Vec<UnsafeVal>;

#[repr(transparent)]
#[derive(Copy, Clone, Debug, Default)]
pub struct Val<'a> {
    inner: UnsafeVal,
    _lifetime: PhantomData<&'a ()>,
}

impl Val<'static> {
    pub fn new_void() -> Val<'static> {
        // Unsafe OK: Void does not have a lifetime.
        unsafe { Self::from_unsafe_val(().into()) }
    }

    pub fn new_bool(x: bool) -> Val<'static> {
        // Unsafe OK: Void does not have a lifetime.
        unsafe { Self::from_unsafe_val(x.into()) }
    }

    pub fn new_int(x: i64) -> Val<'static> {
        // Unsafe OK: Void does not have a lifetime.
        unsafe { Self::from_unsafe_val(x.into()) }
    }

    pub fn new_float(x: f64) -> Val<'static> {
        // Unsafe OK: Void does not have a lifetime.
        unsafe { Self::from_unsafe_val(x.into()) }
    }
}

impl<'a> Val<'a> {
    pub fn is_void(self) -> bool {
        matches!(self.inner, UnsafeVal::Void)
    }

    pub fn is_truthy(self) -> bool {
        self.inner.is_truthy()
    }

    pub fn as_bool(self) -> Option<bool> {
        match self.inner {
            UnsafeVal::Bool(x) => x.into(),
            _ => None,
        }
    }

    pub fn as_int(self) -> Option<i64> {
        match self.inner {
            UnsafeVal::Int(x) => x.into(),
            _ => None,
        }
    }

    pub fn as_float(self) -> Option<f64> {
        match self.inner {
            UnsafeVal::Float(x) => x.into(),
            _ => None,
        }
    }

    pub fn try_str(self, vm: &Vm) -> Result<&str, Val<'a>> {
        self.as_str(vm).ok_or_else(|| self)
    }

    pub fn as_str(self, vm: &Vm) -> Option<&str> {
        match self.inner {
            UnsafeVal::String(id) => Some(vm.objects.get_str(id)),
            _ => None,
        }
    }

    pub fn try_list(self, vm: &Vm) -> Result<&[Val], Val<'a>> {
        self.as_list(vm).ok_or_else(|| self)
    }

    pub fn as_list(self, vm: &Vm) -> Option<&[Val]> {
        match self.inner {
            UnsafeVal::List(id) => {
                let list = vm.objects.get_list(id);
                // The VM is borrowed so it is ensured to not garbage collect.
                Some(unsafe { Val::from_unsafe_val_slice(list.as_slice()) })
            }
            _ => None,
        }
    }

    pub fn type_name(self) -> &'static str {
        self.inner.type_name()
    }

    pub fn formatted(self, vm: &Vm) -> impl '_ + std::fmt::Display {
        self.inner.formatted(vm)
    }

    pub fn format_quoted(self, vm: &Vm) -> impl '_ + std::fmt::Display {
        self.inner.format_quoted(vm)
    }

    /// Returns `true` if a custom value is held.
    pub fn is_custom(&self) -> bool {
        matches!(self.inner, UnsafeVal::Custom(_))
    }

    /// Returns the value as a custom type of `T` or [None] if [Self] is not of the given custom
    /// value.
    pub fn as_custom<T: CustomType>(&self, vm: &'a Vm) -> Result<CustomValRef<T>, CustomValError> {
        match self.inner {
            UnsafeVal::Custom(id) => vm.objects.get_custom(id).get(),
            _ => Err(CustomValError::WrongType {
                expected: std::any::type_name::<T>(),
                actual: self.type_name(),
            }),
        }
    }

    /// Returns the value as a custom type of `T` or [None] if [Self] is not of the given custom
    /// value.
    pub fn as_custom_mut<T: CustomType>(
        &self,
        vm: &'a Vm,
    ) -> Result<CustomValMut<T>, CustomValError> {
        match self.inner {
            UnsafeVal::Custom(id) => vm.objects.get_custom(id).get_mut(),
            _ => Err(CustomValError::WrongType {
                expected: std::any::type_name::<T>(),
                actual: self.type_name(),
            }),
        }
    }
}

impl<'a> Val<'a> {
    pub unsafe fn from_unsafe_val(v: UnsafeVal) -> Val<'a> {
        Val {
            inner: v,
            _lifetime: PhantomData,
        }
    }

    pub unsafe fn from_unsafe_val_slice(v: &'a [UnsafeVal]) -> &'a [Val<'a>] {
        // This is always true as Val is repr(transparent) with an [UnsafeVal] under the hood.
        debug_assert_eq!(
            std::mem::size_of::<UnsafeVal>(),
            std::mem::size_of::<Val<'a>>()
        );
        std::mem::transmute(v)
    }

    pub unsafe fn as_unsafe_val(self) -> UnsafeVal {
        self.inner
    }

    pub unsafe fn as_static(self) -> Val<'static> {
        Val {
            inner: self.inner,
            _lifetime: PhantomData,
        }
    }

    pub unsafe fn as_unsafe_val_slice<'b>(slice: &'b [Val<'a>]) -> &'b [UnsafeVal] {
        // This is always true as Val is repr(transparent) with an [UnsafeVal] under the hood.
        debug_assert_eq!(
            std::mem::size_of::<UnsafeVal>(),
            std::mem::size_of::<Val<'a>>()
        );
        std::mem::transmute(slice)
    }
}
