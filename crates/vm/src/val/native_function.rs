use smol_str::SmolStr;

use crate::{error::VmResult, Vm};

use super::internal::InternalVal;

pub type NativeFunction = fn(NativeFunctionContext) -> VmResult<InternalVal>;

pub struct NativeFunctionContext<'a> {
    vm: &'a mut Vm,
    stack_start: usize,
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

    /// Create a new void value.
    pub fn new_void(&self) -> InternalVal {
        InternalVal::Void
    }

    /// # Safety
    /// The list may be garbage collected if the VM begins its instruction cycle. Safe to call as
    /// final return value call in native function.
    pub unsafe fn new_list(&mut self, list: Vec<InternalVal>) -> InternalVal {
        InternalVal::List(self.vm.val_store.insert_list(list))
    }

    /// # Safety
    /// The string may be garbage collected if the VM begins its instruction cycle. Safe to call as
    /// final return value call in native function.
    pub unsafe fn new_string(&mut self, string: SmolStr) -> InternalVal {
        InternalVal::String(self.vm.val_store.insert_string(string))
    }
}
