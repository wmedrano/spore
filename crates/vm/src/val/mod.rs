//! Contains representation of values within the Spore machine. Each type has differing lifetime and
//! safety guarantees.
mod bytecode;
pub(crate) mod custom;
mod formatter;
mod id;
mod native_function;
mod protected_val;
mod struct_val;
mod symbol;
mod unsafe_val;

use std::marker::PhantomData;

pub use bytecode::{ByteCode, Instruction};
pub use custom::{CustomType, CustomVal, CustomValError, CustomValMut, CustomValRef};
pub use formatter::ValFormatter;
pub use id::ValId;
pub use native_function::{NativeFunction, NativeFunctionContext, ValBuilder};
pub use protected_val::ProtectedVal;
pub use struct_val::StructVal;
pub use symbol::Symbol;
pub use unsafe_val::UnsafeVal;

use crate::Vm;

/// A container for a list.
pub type ListVal = Vec<UnsafeVal>;

/// Contains a [Val] from the [Vm].
#[repr(transparent)]
#[derive(Copy, Clone, Default)]
pub struct Val<'a> {
    inner: UnsafeVal,
    _lifetime: PhantomData<&'a ()>,
}

impl<'a> std::fmt::Debug for Val<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Val").field(&self.inner).finish()
    }
}

impl Val<'static> {
    /// Create a new `void` value.
    pub fn new_void() -> Val<'static> {
        // Unsafe OK: Void does not have a lifetime.
        unsafe { Self::from_unsafe_val(().into()) }
    }

    /// Create a new `bool` value.
    pub fn new_bool(x: bool) -> Val<'static> {
        // Unsafe OK: Void does not have a lifetime.
        unsafe { Self::from_unsafe_val(x.into()) }
    }

    /// Create a new `int` value.
    pub fn new_int(x: i64) -> Val<'static> {
        // Unsafe OK: Void does not have a lifetime.
        unsafe { Self::from_unsafe_val(x.into()) }
    }

    /// Create a new `float` value.
    pub fn new_float(x: f64) -> Val<'static> {
        // Unsafe OK: Void does not have a lifetime.
        unsafe { Self::from_unsafe_val(x.into()) }
    }
}

impl<'a> Val<'a> {
    /// Returns `true` if `self` is a void value.
    pub fn is_void(self) -> bool {
        matches!(self.inner, UnsafeVal::Void)
    }

    /// Returns `true` if `self` is *not* `false` or `void`. Only `false` and `void` will return
    /// `false.`
    pub fn is_truthy(self) -> bool {
        self.inner.is_truthy()
    }

    /// Get the underlying `bool` value or `Err(self)` if `self` is not a bool.
    pub fn try_bool(self) -> Result<bool, Self> {
        match self.inner {
            UnsafeVal::Bool(x) => Ok(x),
            _ => Err(self),
        }
    }

    /// Get the underlying `int` value or `Err(self)` if `self` is not an int.
    pub fn try_int(self) -> Result<i64, Self> {
        match self.inner {
            UnsafeVal::Int(x) => Ok(x),
            _ => Err(self),
        }
    }

    /// Get the underlying `float` value or `Err(self)` if `self` is not a float.
    pub fn try_float(self) -> Result<f64, Self> {
        match self.inner {
            UnsafeVal::Float(x) => Ok(x),
            _ => Err(self),
        }
    }

    /// Get the underlying Symbol or `Err<Val>` if `self` is not a string.
    pub fn try_symbol(self) -> Result<Symbol, Self> {
        match self.inner {
            UnsafeVal::Symbol(sym) => Ok(sym),
            _ => Err(self),
        }
    }

    /// Get the underlying [&str] or `Err<Val>` if `self` is not a string.
    pub fn try_str(self, vm: &Vm) -> Result<&str, Self> {
        match self.inner {
            UnsafeVal::String(id) => Ok(vm.objects.get_str(id)),
            _ => Err(self),
        }
    }

    /// Get the underlying list or `Err<Val>` if `self` is not a list.
    pub fn try_list(self, vm: &Vm) -> Result<&[Val], Val<'a>> {
        match self.inner {
            UnsafeVal::List(id) => {
                let list = vm.objects.get_list(id);
                // The VM is borrowed so it is ensured to not garbage collect.
                Ok(unsafe { Val::from_unsafe_val_slice(list.as_slice()) })
            }
            _ => Err(self),
        }
    }

    /// Returns `true` if `self` is a struct.
    pub fn is_struct(self) -> bool {
        matches!(self.inner, UnsafeVal::Struct(_))
    }

    /// Get the underlying struct or `Err<Val>` if `self` is not a struct.
    pub fn try_struct(self, vm: &Vm) -> Result<&StructVal, Val<'a>> {
        match self.inner {
            UnsafeVal::Struct(id) => {
                let strct = vm.objects.get_struct(id);
                // The VM is borrowed so it is ensured to not garbage collect.
                Ok(strct)
            }
            _ => Err(self),
        }
    }

    /// Get the underlying struct or `Err<Val>` if `self` is not a struct.
    pub fn try_struct_get(self, vm: &'a Vm, name: &str) -> Result<Option<Val<'a>>, Val<'a>> {
        match self.inner {
            UnsafeVal::Struct(id) => {
                let strct = vm.objects.get_struct(id);
                let sym = vm.get_symbol(name).ok_or(self)?;
                let maybe_v = strct.get(sym).map(|v| unsafe { Val::from_unsafe_val(v) });
                Ok(maybe_v)
            }
            _ => Err(self),
        }
    }

    /// Get the underlying struct or `Err<Val>` if `self` is not a struct.
    ///
    /// # Safety
    /// Provides raw access to the struct val.
    pub unsafe fn try_unsafe_struct_mut(self, vm: &mut Vm) -> Result<&mut StructVal, Val<'a>> {
        match self.inner {
            UnsafeVal::Struct(id) => {
                let strct = vm.objects.get_struct_mut(id);
                Ok(strct)
            }
            _ => Err(self),
        }
    }

    /// Get the [Val] that the mutable box is pointing to or `Err<Val>` if `self` is not a mutable
    /// box.
    pub fn try_mutable_box_ref(self, vm: &Vm) -> Result<Val, Val<'a>> {
        match self.inner {
            UnsafeVal::MutableBox(id) => {
                let mutable_box = vm.objects.get_mutable_box(id);
                // The VM is borrowed so it is ensured to not garbage collect.
                Ok(unsafe { Val::from_unsafe_val(*mutable_box) })
            }
            _ => Err(self),
        }
    }

    /// Get the display name of the type held by `self`.
    pub fn type_name(self) -> &'static str {
        self.inner.type_name()
    }

    /// Get a formatter for the underlying type.
    pub fn formatted(self, vm: &Vm) -> impl '_ + std::fmt::Display {
        self.inner.formatted(vm)
    }

    /// Get a formatter for the underlying type.
    ///
    /// Compared to [Self::formatted], `strings` are displayed with quotes around them.
    pub fn format_quoted(self, vm: &Vm) -> impl '_ + std::fmt::Display {
        self.inner.format_quoted(vm)
    }

    /// Returns `true` if a custom value is held.
    pub fn is_custom(&self) -> bool {
        matches!(self.inner, UnsafeVal::Custom(_))
    }

    /// Returns the value as a custom type of `T` or [None] if [Self] is not of the given custom
    /// value.
    pub fn try_custom<T: CustomType>(&self, vm: &'a Vm) -> Result<CustomValRef<T>, CustomValError> {
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
    pub fn try_custom_mut<T: CustomType>(
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
    /// Create a new [Val] from an [UnsafeVal].
    ///
    /// # Safety
    /// [UnsafeVal] must be a valid value. It is possible to [UnsafeVal] to become invalidated
    /// through VM garbage collection.
    pub unsafe fn from_unsafe_val(v: UnsafeVal) -> Val<'a> {
        Val {
            inner: v,
            _lifetime: PhantomData,
        }
    }

    /// Return the underlying [UnsafeVal] representation.
    pub fn as_unsafe_val(self) -> UnsafeVal {
        self.inner
    }

    /// Extend the lifetime of [Val] to `'static`.
    ///
    /// # Safety
    /// This is unsafe as the caller must be certain that [Val] will not be garbage collected.
    pub unsafe fn as_static(self) -> Val<'static> {
        Val {
            inner: self.inner,
            _lifetime: PhantomData,
        }
    }

    /// Convert a slice of [UnsafeVal] to a slice of [Val].
    ///
    /// # Safety
    /// All [UnsafeVal] within `slice` must be valid. It is possible for [UnsafeVal] to become
    /// invalidated through VM garbage collection.
    pub unsafe fn from_unsafe_val_slice(v: &'a [UnsafeVal]) -> &'a [Val<'a>] {
        // This is always true as Val is repr(transparent) with an [UnsafeVal] under the hood.
        debug_assert_eq!(
            std::mem::size_of::<UnsafeVal>(),
            std::mem::size_of::<Val<'a>>()
        );
        std::mem::transmute(v)
    }

    /// Convert a slice of [Val] to a slice of [UnsafeVal].
    pub fn as_unsafe_val_slice<'b>(slice: &'b [Val<'a>]) -> &'b [UnsafeVal] {
        // This is always true as Val is repr(transparent) with an [UnsafeVal] under the hood.
        debug_assert_eq!(
            std::mem::size_of::<UnsafeVal>(),
            std::mem::size_of::<Val<'a>>()
        );
        unsafe { std::mem::transmute(slice) }
    }
}

macro_rules! to_val_impl {
    ($rust_type:ty) => {
        /// Convert from a simple static Rust value into a [Val].
        impl From<$rust_type> for Val<'static> {
            fn from(v: $rust_type) -> Val<'static> {
                unsafe { Self::from_unsafe_val(v.into()) }
            }
        }
    };
}

to_val_impl!(());
to_val_impl!(bool);
to_val_impl!(i64);
to_val_impl!(f64);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_values_can_be_created_outside_of_vm() {
        assert_eq!(
            Val::new_void().formatted(&Vm::default()).to_string(),
            "<void>"
        );
        assert_eq!(
            Val::new_bool(true).formatted(&Vm::default()).to_string(),
            "true"
        );
        assert_eq!(Val::new_int(1).formatted(&Vm::default()).to_string(), "1");
        assert_eq!(
            Val::new_float(2.5).formatted(&Vm::default()).to_string(),
            "2.5"
        );
    }
}
