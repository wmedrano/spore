use std::marker::PhantomData;

use smol_str::SmolStr;

use crate::{error::VmResult, Vm};

use super::{custom::CustomVal, internal::InternalVal, ListVal};

pub type NativeFunction = for<'a> fn(NativeFunctionContext<'a>) -> VmResult<ValBuilder<'a>>;

pub struct NativeFunctionContext<'a> {
    vm: &'a mut Vm,
    stack_start: usize,
}

#[derive(Debug)]
pub struct ValBuilder<'a> {
    val: InternalVal,
    _lt: PhantomData<&'a InternalVal>,
}

impl ValBuilder<'static> {
    pub fn new_void() -> ValBuilder<'static> {
        ValBuilder {
            val: ().into(),
            _lt: PhantomData,
        }
    }

    pub fn new_bool(x: bool) -> ValBuilder<'static> {
        ValBuilder {
            val: x.into(),
            _lt: PhantomData,
        }
    }

    pub fn new_int(x: i64) -> ValBuilder<'static> {
        ValBuilder {
            val: x.into(),
            _lt: PhantomData,
        }
    }

    pub fn new_float(x: f64) -> ValBuilder<'static> {
        ValBuilder {
            val: x.into(),
            _lt: PhantomData,
        }
    }
}

impl<'a> ValBuilder<'a> {
    /// # Safety
    /// The garbage collector may clean up the value. This value must be discarded or inserted into
    /// the VM immediately.
    pub(crate) unsafe fn build(self) -> InternalVal {
        self.val
    }
}

impl<'a> NativeFunctionContext<'a> {
    /// # Safety
    /// Stack start must be less than or equal to the Vm's stack length.
    pub fn new(vm: &mut Vm, stack_start: usize) -> NativeFunctionContext {
        NativeFunctionContext { vm, stack_start }
    }

    /// Get the underlying VM.
    pub fn vm(&self) -> &Vm {
        self.vm
    }

    /// Get the arguments to the function call.
    pub fn args(&self) -> &[InternalVal] {
        &self.vm.stack[self.stack_start..]
    }

    /// Get the number of arguments passed in to the function call.
    pub fn arg_len(&self) -> usize {
        self.vm.stack.len() - self.stack_start
    }

    /// # Safety
    /// Garbage collector may clean up this value. For safety, the value must be returned
    /// immediately.
    pub unsafe fn new_string(&mut self, s: SmolStr) -> ValBuilder<'a> {
        let string_id = self.vm.val_store.insert_string(s);
        ValBuilder {
            val: string_id.into(),
            _lt: PhantomData,
        }
    }

    /// # Safety
    /// Garbage collector may clean up this value. For safety, the value must be returned
    /// immediately.
    pub unsafe fn new_list(&mut self, list: ListVal) -> ValBuilder<'a> {
        let list_id = self.vm.val_store.insert_list(list);
        ValBuilder {
            val: list_id.into(),
            _lt: PhantomData,
        }
    }

    /// # Safety
    /// Garbage collector may clean up this value. For safety, the value must be returned
    /// immediately.
    pub unsafe fn new_custom(&mut self, c: CustomVal) -> ValBuilder<'a> {
        let custom_id = self.vm.val_store.insert_custom(c);
        ValBuilder {
            val: custom_id.into(),
            _lt: PhantomData,
        }
    }
}
