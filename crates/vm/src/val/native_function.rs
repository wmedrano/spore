use std::marker::PhantomData;

use compact_str::CompactString;

use crate::{error::VmResult, Vm};

use super::{
    custom::{CustomType, CustomVal},
    internal::UnsafeVal,
    ListVal, Val,
};

pub type NativeFunction = for<'a> fn(NativeFunctionContext<'a>) -> VmResult<ValBuilder<'a>>;

pub struct NativeFunctionContext<'a> {
    vm: &'a mut Vm,
    stack_start: usize,
}

#[derive(Debug)]
pub struct ValBuilder<'a> {
    val: UnsafeVal,
    _lt: PhantomData<&'a UnsafeVal>,
}

impl ValBuilder<'static> {
    /// Create a new value from an internal.
    ///
    /// # Safety
    /// `InternalVal` must be a valid value that has not been garbage collected.
    pub unsafe fn new_internal(v: UnsafeVal) -> ValBuilder<'static> {
        ValBuilder {
            val: v,
            _lt: PhantomData,
        }
    }

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
    pub(crate) unsafe fn build(self) -> UnsafeVal {
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

    /// Get the underlying VM.
    ///
    /// # Safety
    /// Having access to the mutable VM gives a lot of power, use it responsibly.
    pub fn vm_mut(&mut self) -> &mut Vm {
        self.vm
    }

    /// Get the argument as a val.
    pub fn arg(&mut self, idx: usize) -> Val {
        let v = self.args()[idx];
        Val::new(self.vm, v)
    }

    /// Get the arguments to the function call.
    pub fn args(&self) -> &[UnsafeVal] {
        &self.vm.stack[self.stack_start..]
    }

    /// Get the number of arguments passed in to the function call.
    pub fn arg_len(&self) -> usize {
        self.vm.stack.len() - self.stack_start
    }

    /// # Safety
    /// Garbage collector may clean up this value. For safety, the value must be returned
    /// immediately.
    pub unsafe fn new_string(&mut self, s: CompactString) -> ValBuilder<'a> {
        let string_id = self.vm.objects.insert_string(s);
        ValBuilder {
            val: string_id.into(),
            _lt: PhantomData,
        }
    }

    /// # Safety
    /// Garbage collector may clean up this value. For safety, the value must be returned
    /// immediately.
    pub unsafe fn new_mutable_box(&mut self, v: UnsafeVal) -> ValBuilder<'a> {
        let id = self.vm.objects.insert_mutable_box(v);
        ValBuilder {
            val: id.into(),
            _lt: PhantomData,
        }
    }

    /// # Safety
    /// Garbage collector may clean up this value. For safety, the value must be returned
    /// immediately.
    pub unsafe fn new_list(&mut self, list: ListVal) -> ValBuilder<'a> {
        let list_id = self.vm.objects.insert_list(list);
        ValBuilder {
            val: list_id.into(),
            _lt: PhantomData,
        }
    }

    /// # Safety
    /// Garbage collector may clean up this value. For safety, the value must be returned
    /// immediately.
    pub unsafe fn new_custom(&mut self, obj: impl CustomType) -> ValBuilder<'a> {
        let custom_val = CustomVal::new(obj);
        let custom_id = self.vm.objects.insert_custom(custom_val);
        ValBuilder {
            val: custom_id.into(),
            _lt: PhantomData,
        }
    }
}
