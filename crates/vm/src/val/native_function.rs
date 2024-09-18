use smol_str::SmolStr;

use crate::{error::VmResult, Vm};

use super::internal::InternalVal;

pub type NativeFunction = fn(NativeFunctionContext) -> VmResult<ValBuilder>;

pub struct NativeFunctionContext<'a> {
    vm: &'a mut Vm,
    stack_start: usize,
}

#[derive(Debug)]
enum ValBuilderImpl {
    Literal(InternalVal),
    String(SmolStr),
}

#[derive(Debug)]
pub struct ValBuilder(ValBuilderImpl);

impl ValBuilder {
    pub fn new_void() -> ValBuilder {
        ValBuilder(ValBuilderImpl::Literal(InternalVal::Void))
    }

    pub fn new_bool(x: bool) -> ValBuilder {
        ValBuilder(ValBuilderImpl::Literal(InternalVal::Bool(x)))
    }

    pub fn new_int(x: i64) -> ValBuilder {
        ValBuilder(ValBuilderImpl::Literal(InternalVal::Int(x)))
    }

    pub fn new_float(x: f64) -> ValBuilder {
        ValBuilder(ValBuilderImpl::Literal(InternalVal::Float(x)))
    }

    pub fn new_string(x: SmolStr) -> ValBuilder {
        ValBuilder(ValBuilderImpl::String(x))
    }

    pub(crate) fn to_internal(self, vm: &mut Vm) -> InternalVal {
        match self.0 {
            ValBuilderImpl::Literal(v) => v,
            ValBuilderImpl::String(v) => InternalVal::String(vm.val_store.insert_string(v)),
        }
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
    /// The list may be garbage collected if the VM begins its instruction cycle. Safe to call as
    /// final return value call in native function.
    pub unsafe fn new_list(&mut self, list: Vec<InternalVal>) -> ValBuilder {
        ValBuilder(ValBuilderImpl::Literal(InternalVal::List(
            self.vm.val_store.insert_list(list),
        )))
    }
}
