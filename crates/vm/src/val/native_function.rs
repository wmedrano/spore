use compact_str::CompactString;

use crate::{error::VmResult, Vm};

use super::{
    custom::{CustomType, CustomVal},
    UnsafeVal, Val,
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
///
/// ```rust
/// fn my_magic_string(ctx: spore_vm::val::NativeFunctionContext) -> spore_vm::error::VmResult<spore_vm::val::ValBuilder> {
///     Ok(ctx.new_string("42".into()))
/// }
pub type NativeFunction = for<'a> fn(NativeFunctionContext<'a>) -> VmResult<ValBuilder<'a>>;

/// Builds a value suitable for return returning.
///
/// - This is often returned by [NativeFunction].
/// - `ValBuilder` objects may be built from [NativeFunctionContext] objects.
///
/// ```rust
/// fn my_magic_string(ctx: spore_vm::val::NativeFunctionContext) -> spore_vm::error::VmResult<spore_vm::val::ValBuilder> {
///     Ok(ctx.new_string("42".into()))
/// }
/// ```
#[derive(Debug)]
pub struct ValBuilder<'a> {
    val: Val<'a>,
}

impl ValBuilder<'static> {
    /// Create a new `ValBuilder` from a static [Val].
    ///
    /// ```rust
    /// spore_vm::val::ValBuilder::new(().into());   // void
    /// spore_vm::val::ValBuilder::new(true.into()); // bool
    /// spore_vm::val::ValBuilder::new(0i64.into()); // int
    /// spore_vm::val::ValBuilder::new(0.0.into());  // float
    /// ```
    pub fn new(val: Val<'static>) -> ValBuilder<'static> {
        ValBuilder { val }
    }
}

impl From<Val<'static>> for ValBuilder<'static> {
    fn from(val: Val) -> ValBuilder {
        ValBuilder { val }
    }
}

/// The input parameter to native Spore VM functions registered with [Vm::with_native_function].
///
/// ```rust
/// fn my_magic_string(ctx: spore_vm::val::NativeFunctionContext) -> spore_vm::error::VmResult<spore_vm::val::ValBuilder> {
///     Ok(ctx.new_string("42".into()))
/// }
pub struct NativeFunctionContext<'a> {
    /// The Vm for the native function.
    ///
    /// # Safety
    /// Do not run anything that may remove references or call the garbage collector.
    vm: &'a mut Vm,
    /// Where the stack starts for the current function call.
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

    /// Get a mutable reference to the underlying VM.
    ///
    /// # Safety
    /// Any operations that triger GC or evaluation will cause undefined behavior.
    pub unsafe fn vm_mut(&mut self) -> &mut Vm {
        self.vm
    }

    /// Get the argument as a [crate::Val]. If the argument is out of range, then [None] is
    /// returned.
    pub fn arg(&self, idx: usize) -> Option<Val<'a>> {
        let v = self.vm.stack.get(self.stack_start + idx)?;
        // Unsafe OK: Args are from the VM's stack so they are not garbage collected.
        Some(unsafe { Val::from_unsafe_val(*v) })
    }

    /// Get the arguments to the function call.
    pub fn args(&self) -> &[Val] {
        // Unsafe OK: Args are from the VM's stack so they are not garbage collected.
        unsafe { Val::from_unsafe_val_slice(self.raw_args()) }
    }

    /// Get the arguments to the function call.
    ///
    /// This is like [Self::args], but returns self as well.
    pub fn split_args(self) -> (Self, &'a [Val<'a>]) {
        // Unsafe OK: Args are from the VM's stack so they are not garbage collected.
        let args = unsafe {
            let args = self.args();
            std::slice::from_raw_parts(args.as_ptr().cast::<Val<'static>>(), args.len())
        };
        (self, args)
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
    ///
    /// ```rust
    /// fn my_magic_string(ctx: spore_vm::val::NativeFunctionContext) -> spore_vm::error::VmResult<spore_vm::val::ValBuilder> {
    ///     Ok(ctx.new_string("42".into()))
    /// }
    /// ```
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
    pub unsafe fn new_list(self, list: &[Val]) -> ValBuilder<'a> {
        // Unsafe OK: Will be inserting into VM.
        let unsafe_list = Val::as_unsafe_val_slice(list);
        let list_id = self.vm.objects.insert_list(unsafe_list.to_vec());
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
