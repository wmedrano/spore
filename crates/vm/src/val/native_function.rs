use compact_str::CompactString;

use crate::{error::VmResult, Vm};

use super::{
    custom::{CustomType, CustomVal},
    ListVal, UnsafeVal, Val,
};

/// A function that can be executed by the Spore VM. Native functions can be registered with
/// [Vm::with_native_function].
///
/// # Argument
/// Native functions take a [NativeFunctionContext] as an argument. This contains the state of the
/// VM.
///
/// # Return Value
/// VmResult<[ValBuilder]> is used to build a return value and insert it into the VM.
pub type NativeFunction = for<'a> fn(NativeFunctionContext<'a>) -> VmResult<ValBuilder<'a>>;

/// Builds a value suitable for return from a [NativeFunction]. `ValBuilder` objects may be built
/// from [NativeFunctionContext] objects.
#[derive(Debug)]
pub struct ValBuilder<'a> {
    val: Val<'a>,
}

impl ValBuilder<'static> {
    pub fn new(val: Val<'static>) -> ValBuilder {
        ValBuilder { val }
    }
}

impl From<Val<'static>> for ValBuilder<'static> {
    fn from(val: Val) -> ValBuilder {
        ValBuilder { val }
    }
}

/// The input parameter to native Spore VM functions registered with [Vm::with_native_function].
pub struct NativeFunctionContext<'a> {
    vm: &'a mut Vm,
    stack_start: usize,
}

impl<'a> NativeFunctionContext<'a> {
    /// # Safety
    /// - Stack start must be less than or equal to the Vm's stack length.
    pub(crate) fn new(vm: &mut Vm, stack_start: usize) -> NativeFunctionContext {
        NativeFunctionContext { vm, stack_start }
    }

    /// Get the underlying VM.
    pub fn vm(&self) -> &Vm {
        self.vm
    }

    /// Get the underlying VM.
    pub fn vm_mut(&mut self) -> &mut Vm {
        self.vm
    }

    /// Get the argument as a [ProtectedVal] that is guaranteed to not be garbage collected.
    pub fn arg(&self, idx: usize) -> Val<'a> {
        let v = self.vm.stack[self.stack_start + idx];
        // Unsafe OK: Args are from the VM's stack so they are not garbage collected.
        unsafe { Val::from_unsafe_val(v) }
    }

    /// Get the arguments to the function call.
    ///
    /// All values returned are guaranteed to live for the rest of the scope and not be garbage
    /// collected.
    pub fn args(&self) -> &[Val] {
        // Unsafe OK: Args are from the VM's stack so they are not garbage collected.
        unsafe { Val::from_unsafe_val_slice(self.raw_args()) }
    }

    /// Get the arguments to the function call.
    ///
    /// All values returned are guaranteed to live for the rest of the scope and not be garbage
    /// collected.
    ///
    /// # Safety
    /// Prefer using [Self::args] which provides values with valid lifetimes.
    pub unsafe fn raw_args(&self) -> &[UnsafeVal] {
        &self.vm.stack[self.stack_start..]
    }

    /// Get the number of arguments passed in to the function call.
    pub fn args_len(&self) -> usize {
        self.vm.stack.len() - self.stack_start
    }
}

impl<'a> NativeFunctionContext<'a> {
    /// Create a new value from an internal.
    ///
    /// Consumes the `self` to ensure that the value isn't garbage collected.
    ///
    /// # Safety
    /// `InternalVal` must be a valid value that has not been garbage collected.
    pub unsafe fn with_unsafe_val(self, val: UnsafeVal) -> ValBuilder<'a> {
        ValBuilder {
            val: Val::from_unsafe_val(val),
        }
    }

    /// Create a new `string` value.
    ///
    /// Consumes `self` to ensure that the value isn't garbage collected.
    pub fn new_string(self, s: CompactString) -> ValBuilder<'a> {
        let string_id = self.vm.objects.insert_string(s);
        ValBuilder {
            // Unsafe OK: String was just created so it does not have a chance to garbage collect.
            val: unsafe { Val::from_unsafe_val(string_id.into()) },
        }
    }

    /// Create a new box from the unsafe val.
    ///
    /// Consumes the `self` to ensure that the value isn't garbage collected.
    ///
    /// # Safety
    /// `v` must be a valid value within the vm.
    pub unsafe fn new_mutable_box(self, v: Val<'a>) -> ValBuilder<'a> {
        let id = self.vm.objects.insert_mutable_box(v.as_unsafe_val());
        ValBuilder {
            // Unsafe OK: Box is just created so it does not have a chance to garbage collect.
            val: Val::from_unsafe_val(id.into()),
        }
    }

    /// Create a new list from `ListVal`.
    ///
    /// Consumes the self to ensure that the value isn't garbage collected.
    ///
    /// # Safety
    /// `list` must contain valid values within the vm.
    pub unsafe fn new_list(self, list: ListVal) -> ValBuilder<'a> {
        let list_id = self.vm.objects.insert_list(list);
        ValBuilder {
            val: Val::from_unsafe_val(list_id.into()),
        }
    }

    /// Create a new custom value from `obj`.
    pub fn new_custom(self, obj: impl CustomType) -> ValBuilder<'a> {
        let custom_val = CustomVal::new(obj);
        let custom_id = self.vm.objects.insert_custom(custom_val);
        ValBuilder {
            // Unsafe OK: Custom is just created so it does not have a chance to garbage collect.
            val: unsafe { Val::from_unsafe_val(custom_id.into()) },
        }
    }
}

impl<'a> ValBuilder<'a> {
    /// # Safety
    /// The garbage collector may clean up the value. This value must be discarded or inserted into
    /// the VM immediately.
    pub(crate) unsafe fn build(self) -> UnsafeVal {
        self.val.inner
    }
}
