use crate::Vm;

use super::InternalVal;

#[derive(Debug)]
pub struct Val<'a> {
    pub(crate) vm: &'a mut Vm,
    pub(crate) v: InternalVal,
    _internal: (),
}

impl<'a> Val<'a> {
    pub(crate) fn new(vm: &'a mut Vm, v: InternalVal) -> Val<'a> {
        vm.val_store.keep_alive(v);
        Val {
            vm,
            v,
            _internal: (),
        }
    }
}

impl<'a> Drop for Val<'a> {
    fn drop(&mut self) {
        self.vm.val_store.allow_death(self.v);
    }
}

impl<'a> Val<'a> {
    pub fn is_void(&self) -> bool {
        matches!(self.v, InternalVal::Void)
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self.v {
            InternalVal::Bool(x) => Some(x),
            _ => None,
        }
    }

    pub fn as_int(&self) -> Option<i64> {
        match self.v {
            InternalVal::Int(x) => Some(x),
            _ => None,
        }
    }

    pub fn as_float(&self) -> Option<f64> {
        match self.v {
            InternalVal::Float(x) => Some(x),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self.v {
            InternalVal::String(x) => Some(self.vm.val_store.get_str(x)),
            _ => None,
        }
    }
}

impl<'a> std::fmt::Display for Val<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.v.formatted(self.vm).fmt(f)
    }
}
