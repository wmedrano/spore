mod bytecode;
pub(crate) mod custom;
mod formatter;
mod id;
pub(crate) mod internal;
mod native_function;

pub use bytecode::{ByteCode, Instruction};
use custom::CustomType;
pub use formatter::ValFormatter;
pub use id::ValId;
use internal::InternalValImpl;
pub use internal::{InternalVal, ListVal};
pub use native_function::{NativeFunction, NativeFunctionContext, ValBuilder};

use crate::Vm;

#[derive(Debug)]
pub struct Val<'a> {
    pub(crate) vm: &'a mut Vm,
    pub(crate) v: InternalVal,
    _internal: (),
}

impl<'a> Val<'a> {
    pub(crate) fn new(vm: &'a mut Vm, v: InternalVal) -> Val<'a> {
        vm.val_store.keep_reachable(v);
        Val {
            vm,
            v,
            _internal: (),
        }
    }
}

impl<'a> Drop for Val<'a> {
    fn drop(&mut self) {
        self.vm.val_store.allow_unreachable(self.v);
    }
}

impl<'a> Val<'a> {
    pub fn is_void(&self) -> bool {
        matches!(self.v.0, InternalValImpl::Void)
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self.v.0 {
            InternalValImpl::Bool(x) => Some(x),
            _ => None,
        }
    }

    pub fn as_int(&self) -> Option<i64> {
        match self.v.0 {
            InternalValImpl::Int(x) => Some(x),
            _ => None,
        }
    }

    pub fn as_float(&self) -> Option<f64> {
        match self.v.0 {
            InternalValImpl::Float(x) => Some(x),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self.v.0 {
            InternalValImpl::String(x) => Some(self.vm.val_store.get_str(x)),
            _ => None,
        }
    }

    pub fn as_custom<T: CustomType>(&self) -> Option<&T> {
        match self.v.0 {
            InternalValImpl::Custom(id) => self.vm.val_store.get_custom(id).get(),
            _ => None,
        }
    }
}

impl<'a> std::fmt::Display for Val<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.v.formatted(self.vm).fmt(f)
    }
}
