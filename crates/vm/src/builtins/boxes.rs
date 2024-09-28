use crate::{
    error::{VmError, VmResult},
    val::{NativeFunctionContext, UnsafeVal, Val, ValBuilder},
};

pub fn new_box<'a>(ctx: NativeFunctionContext<'a>, args: &[Val]) -> VmResult<ValBuilder<'a>> {
    match args {
        [v] => {
            // Unsafe OK: `ctx.args()` guarantees objects that will not be garbage collected.
            let v = unsafe { v.as_static() };
            Ok(unsafe { ctx.new_mutable_box(v) })
        }
        args => Err(VmError::ArityError {
            function: "new-box".into(),
            expected: 1,
            actual: args.len(),
        }),
    }
}

pub fn set_box<'a>(mut ctx: NativeFunctionContext<'a>, args: &[Val]) -> VmResult<ValBuilder<'a>> {
    // TODO: Use safe API.
    match unsafe { Val::as_unsafe_val_slice(args) } {
        // Unsafe OK: This is for sure safe...
        [UnsafeVal::MutableBox(id), inner_val] => {
            let (id, inner_val) = (*id, *inner_val);
            // Unsafe OK: Defining new value and returning right away.
            let boxed_val = unsafe { ctx.vm_mut().objects.set_mutable_box(id, inner_val) };
            // Unsafe OK: `boxed_val` has just been created so it will not be garbage collected.
            Ok(unsafe { ctx.with_unsafe_val(boxed_val) })
        }
        [arg, _] => Err(VmError::TypeError {
            context: "set-box!",
            expected: UnsafeVal::MUTABLE_BOX_TYPE_NAME,
            actual: arg.type_name(),
            value: arg.format_quoted(ctx.vm()).to_string(),
        }),
        args => Err(VmError::ArityError {
            function: "set-box!".into(),
            expected: 2,
            actual: args.len(),
        }),
    }
}

pub fn unbox<'a>(ctx: NativeFunctionContext<'a>, args: &[Val]) -> VmResult<ValBuilder<'a>> {
    // TODO: Use safe API.
    match unsafe { Val::as_unsafe_val_slice(args) } {
        [UnsafeVal::MutableBox(id)] => {
            let boxed_val = *ctx.vm().objects.get_mutable_box(*id);
            // Unsafe OK: `boxed_val` has just been retrieved so the VM does not have a chance to
            // garbage collect it.
            Ok(unsafe { ctx.with_unsafe_val(boxed_val) })
        }
        [arg] => Err(VmError::TypeError {
            context: "unbox",
            expected: UnsafeVal::MUTABLE_BOX_TYPE_NAME,
            actual: arg.type_name(),
            value: arg.format_quoted(ctx.vm()).to_string(),
        }),
        args => Err(VmError::ArityError {
            function: "unbox".into(),
            expected: 1,
            actual: args.len(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use crate::Vm;

    use super::*;

    #[test]
    fn new_box_with_wrong_args_returns_error() {
        let mut vm = Vm::default();
        assert_eq!(
            vm.eval_str("(new-box)").unwrap_err(),
            VmError::ArityError {
                function: "new-box".into(),
                expected: 1,
                actual: 0
            }
        );
        assert_eq!(
            vm.eval_str("(new-box 0 1)").unwrap_err(),
            VmError::ArityError {
                function: "new-box".into(),
                expected: 1,
                actual: 2
            }
        );
    }

    #[test]
    fn referencing_box_does_not_return_inner_value() {
        let mut vm = Vm::default();
        vm.eval_str("(define val (new-box \"foo\"))").unwrap();
        assert!(vm.eval_str("val").unwrap().try_str().is_err());
    }

    #[test]
    fn get_box_returns_value_inside_box() {
        let mut vm = Vm::default();
        vm.eval_str("(define val (new-box \"foo\"))").unwrap();
        assert_eq!(
            vm.eval_str("(unbox val)").unwrap().try_str().unwrap(),
            "foo"
        );
    }

    #[test]
    fn get_box_with_wrong_args_returns_error() {
        let mut vm = Vm::default();
        assert_eq!(
            vm.eval_str("(unbox)").unwrap_err(),
            VmError::ArityError {
                function: "unbox".into(),
                expected: 1,
                actual: 0
            }
        );
        assert_eq!(
            vm.eval_str("(unbox (new-box 0) 1)").unwrap_err(),
            VmError::ArityError {
                function: "unbox".into(),
                expected: 1,
                actual: 2
            }
        );
        assert_eq!(
            vm.eval_str("(unbox 0)").unwrap_err(),
            VmError::TypeError {
                context: "unbox",
                expected: UnsafeVal::MUTABLE_BOX_TYPE_NAME,
                actual: UnsafeVal::INT_TYPE_NAME,
                value: "0".to_string(),
            }
        );
    }

    #[test]
    fn set_box_changes_value_for_subsequent_get_box_calls() {
        let mut vm = Vm::default();
        vm.eval_str("(define val (new-box \"foo\"))").unwrap();
        assert!(vm.eval_str("val").unwrap().try_str().is_err());
        assert_eq!(
            vm.eval_str("(unbox val)").unwrap().try_str().unwrap(),
            "foo"
        );

        vm.eval_str("(set-box! val \"bar\")").unwrap();
        assert_eq!(
            vm.eval_str("(unbox val)").unwrap().try_str().unwrap(),
            "bar"
        );
    }

    #[test]
    fn set_box_with_wrong_args_returns_error() {
        let mut vm = Vm::default();
        assert_eq!(
            vm.eval_str("(set-box!)").unwrap_err(),
            VmError::ArityError {
                function: "set-box!".into(),
                expected: 2,
                actual: 0
            }
        );
        assert_eq!(
            vm.eval_str("(set-box! (new-box 0))").unwrap_err(),
            VmError::ArityError {
                function: "set-box!".into(),
                expected: 2,
                actual: 1
            }
        );
        assert_eq!(
            vm.eval_str("(set-box! 0 (new-box 0))").unwrap_err(),
            VmError::TypeError {
                context: "set-box!",
                expected: UnsafeVal::MUTABLE_BOX_TYPE_NAME,
                actual: UnsafeVal::INT_TYPE_NAME,
                value: "0".to_string(),
            }
        );
        assert_eq!(
            vm.eval_str("(set-box! (new-box 0) 1 2)").unwrap_err(),
            VmError::ArityError {
                function: "set-box!".into(),
                expected: 2,
                actual: 3
            }
        );
    }
}
