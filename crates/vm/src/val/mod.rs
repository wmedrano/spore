mod bytecode;
pub(crate) mod custom;
mod formatter;
mod id;
pub(crate) mod internal;
mod native_function;

pub use bytecode::{ByteCode, Instruction};
pub use custom::CustomType;
pub use formatter::ValFormatter;
pub use id::ValId;
use internal::InternalValImpl;
pub use internal::{InternalVal, ListVal};
pub use native_function::{NativeFunction, NativeFunctionContext, ValBuilder};

use crate::Vm;

/// Holds a Value from the [Vm].
#[derive(Debug)]
pub struct Val<'a> {
    pub(crate) vm: &'a mut Vm,
    pub(crate) v: InternalVal,
    _internal: (),
}

impl<'a> Val<'a> {
    /// Create a new `Val` from an [InternalVal].
    pub(crate) fn new(vm: &'a mut Vm, v: InternalVal) -> Val<'a> {
        vm.objects.keep_reachable(v);
        Val {
            vm,
            v,
            _internal: (),
        }
    }
}

impl<'a> Drop for Val<'a> {
    fn drop(&mut self) {
        self.vm.objects.allow_unreachable(self.v);
    }
}

impl<'a> Val<'a> {
    /// Returns true if val is void.
    pub fn is_void(&self) -> bool {
        matches!(self.v.0, InternalValImpl::Void)
    }

    /// Returns the value a boolean or [None] if [Self] is not a boolean.
    pub fn as_bool(&self) -> Option<bool> {
        match self.v.0 {
            InternalValImpl::Bool(x) => Some(x),
            _ => None,
        }
    }

    /// Returns the value as an int or [None] if [Self] is not an int.
    pub fn as_int(&self) -> Option<i64> {
        match self.v.0 {
            InternalValImpl::Int(x) => Some(x),
            _ => None,
        }
    }

    /// Returns the value as a float or [None] if [Self] is not a float.
    pub fn as_float(&self) -> Option<f64> {
        match self.v.0 {
            InternalValImpl::Float(x) => Some(x),
            _ => None,
        }
    }

    /// Returns the value as a [str] or [None] if [Self] is not a string.
    pub fn as_str(&self) -> Option<&str> {
        match self.v.0 {
            InternalValImpl::String(x) => Some(self.vm.objects.get_str(x)),
            _ => None,
        }
    }

    /// Returns `true` if a custom value is held.
    pub fn is_custom(&self) -> bool {
        matches!(self.v.0, InternalValImpl::Custom(_))
    }

    /// Returns the value as a custom type of `T` or [None] if [Self] is not of the given custom
    /// value.
    pub fn as_custom<T: CustomType>(&self) -> Option<&T> {
        match self.v.0 {
            InternalValImpl::Custom(id) => self.vm.objects.get_custom(id).get(),
            _ => None,
        }
    }
}

impl<'a> std::fmt::Display for Val<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.v.formatted(self.vm).fmt(f)
    }
}
