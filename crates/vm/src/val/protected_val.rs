use crate::Vm;

use super::{CustomType, UnsafeVal};

/// Holds a Value from the [Vm]. Unlike [UnsafeVal], the underlying value is guaranteed to not be
/// garbage collected.
#[derive(Debug)]
pub struct ProtectedVal<'a> {
    pub(crate) vm: &'a mut Vm,
    pub(crate) v: UnsafeVal,
    _internal: (),
}

impl<'a> ProtectedVal<'a> {
    /// Create a new `Val` from an [UnsafeVal].
    ///
    /// # Safety
    /// `v` must be a valid from originating (and not garbage collected) from `vm`.
    pub unsafe fn new(vm: &'a mut Vm, v: UnsafeVal) -> ProtectedVal<'a> {
        vm.objects.keep_reachable(v);
        ProtectedVal {
            vm,
            v,
            _internal: (),
        }
    }
}

impl<'a> Drop for ProtectedVal<'a> {
    fn drop(&mut self) {
        self.vm.objects.allow_unreachable(self.v);
    }
}

impl<'a> ProtectedVal<'a> {
    /// Returns true if val is void.
    pub fn is_void(&self) -> bool {
        matches!(self.v, UnsafeVal::Void)
    }

    /// Returns the value a boolean or [None] if [Self] is not a boolean.
    pub fn as_bool(&self) -> Option<bool> {
        match self.v {
            UnsafeVal::Bool(x) => Some(x),
            _ => None,
        }
    }

    /// Returns the value as an int or [None] if [Self] is not an int.
    pub fn as_int(&self) -> Option<i64> {
        match self.v {
            UnsafeVal::Int(x) => Some(x),
            _ => None,
        }
    }

    /// Returns the value as a float or [None] if [Self] is not a float.
    pub fn as_float(&self) -> Option<f64> {
        match self.v {
            UnsafeVal::Float(x) => Some(x),
            _ => None,
        }
    }

    /// Returns the value as a [str] or [None] if [Self] is not a string.
    pub fn as_str(&self) -> Option<&str> {
        match self.v {
            UnsafeVal::String(x) => Some(self.vm.objects.get_str(x)),
            _ => None,
        }
    }

    /// Returns `true` if a custom value is held.
    pub fn is_custom(&self) -> bool {
        matches!(self.v, UnsafeVal::Custom(_))
    }

    /// Returns the value as a custom type of `T` or [None] if [Self] is not of the given custom
    /// value.
    pub fn as_custom<T: CustomType>(&self) -> Option<&T> {
        match self.v {
            UnsafeVal::Custom(id) => self.vm.objects.get_custom(id).get(),
            _ => None,
        }
    }
}

impl<'a> std::fmt::Display for ProtectedVal<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.v.formatted(self.vm).fmt(f)
    }
}
