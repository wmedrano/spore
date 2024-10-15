use crate::{
    error::{VmError, VmResult},
    val::{NativeFunctionContext, StructVal, UnsafeVal, ValBuilder},
    Vm,
};

pub fn strct(ctx: NativeFunctionContext<'_>) -> VmResult<ValBuilder<'_>> {
    let mut strct = StructVal::with_capacity(ctx.arg_count() / 2);

    let mut args_iter = ctx.args();
    if ctx.arg_count() % 2 != 0 {
        return Err(VmError::ArityError {
            function: "struct needs an even amount of args, ".into(),
            expected: ctx.arg_count() + 1,
            actual: ctx.arg_count(),
        });
    }
    while let Some(field) = args_iter.next() {
        let field_sym = field.try_symbol().map_err(|v| VmError::TypeError {
            src: None,
            context: "struct field name",
            expected: UnsafeVal::SYMBOL_TYPE_NAME,
            actual: v.type_name(),
            value: v.format_quoted(ctx.vm()).to_string(),
        })?;
        let val = args_iter.next().unwrap();
        strct.set(field_sym, unsafe { val.as_static() });
    }
    drop(args_iter);
    Ok(unsafe { ctx.new_struct(strct) })
}

pub fn struct_get(ctx: NativeFunctionContext<'_>) -> VmResult<ValBuilder<'_>> {
    let vm = ctx.vm();
    match ctx.arg_count() {
        2 => {
            let field = ctx
                .arg(1)
                .unwrap()
                .try_symbol()
                .map_err(|v| VmError::TypeError {
                    src: None,
                    context: "struct-get arg(idx=1)",
                    expected: UnsafeVal::SYMBOL_TYPE_NAME,
                    actual: v.type_name(),
                    value: v.format_quoted(ctx.vm()).to_string(),
                })?;
            let strct = ctx
                .arg(0)
                .unwrap()
                .try_struct(vm)
                .map_err(|v| VmError::TypeError {
                    src: None,
                    context: "struct-get arg(idx=0)",
                    expected: UnsafeVal::STRUCT_TYPE_NAME,
                    actual: v.type_name(),
                    value: v.format_quoted(ctx.vm()).to_string(),
                })?;
            let v = strct.get(field).unwrap_or(UnsafeVal::Void);
            Ok(unsafe { ctx.with_unsafe_val(v) })
        }
        args => Err(VmError::ArityError {
            function: "struct-get".into(),
            expected: 2,
            actual: args,
        }),
    }
}

pub fn struct_set(mut ctx: NativeFunctionContext<'_>) -> VmResult<ValBuilder<'_>> {
    match ctx.arg_count() {
        3 => {
            let field = ctx
                .arg(1)
                .unwrap()
                .try_symbol()
                .map_err(|v| VmError::TypeError {
                    src: None,
                    context: "struct-set! arg(idx=1)",
                    expected: UnsafeVal::SYMBOL_TYPE_NAME,
                    actual: v.type_name(),
                    value: v.format_quoted(ctx.vm()).to_string(),
                })?;
            let vm: *mut Vm = unsafe { ctx.vm_mut() };
            let strct = match unsafe { ctx.arg(0).unwrap().try_unsafe_struct_mut(&mut *vm) } {
                Ok(v) => v,
                Err(v) => {
                    return Err(VmError::TypeError {
                        src: None,
                        context: "struct-set! arg(idx=0)",
                        expected: UnsafeVal::STRUCT_TYPE_NAME,
                        actual: v.type_name(),
                        value: v.format_quoted(ctx.vm()).to_string(),
                    });
                }
            };
            strct.set(field, unsafe { ctx.arg(2).unwrap().as_static() });
            Ok(ValBuilder::new(().into()))
        }
        args => Err(VmError::ArityError {
            function: "struct-set!".into(),
            expected: 3,
            actual: args,
        }),
    }
}

#[cfg(test)]
mod tests {
    use crate::{parser::span::Span, Vm};

    use super::*;

    #[test]
    fn struct_with_no_args_returns_empty_struct() {
        let mut vm = Vm::default();
        assert!(vm
            .eval_str("(= (struct) (struct))")
            .unwrap()
            .try_bool()
            .unwrap());
    }

    #[test]
    fn struct_with_odd_args_returns_error() {
        let mut vm = Vm::default();
        assert_eq!(
            vm.eval_str("(struct 'field)").unwrap_err(),
            VmError::ArityError {
                function: "struct needs an even amount of args, ".into(),
                expected: 2,
                actual: 1
            }
        )
    }

    #[test]
    fn struct_get_with_field_returns_field() {
        let mut vm = Vm::default();
        let got = vm
            .eval_str("(struct-get (struct 'field 1.0) 'field)")
            .unwrap();
        assert_eq!(got.try_float().unwrap(), 1.0);
    }

    #[test]
    fn struct_get_with_field_that_does_not_exist_returns_void() {
        let mut vm = Vm::default();
        let got = vm
            .eval_str("(struct-get (struct 'field 1) 'not-field)")
            .unwrap();
        assert!(got.is_void());
    }

    #[test]
    fn struct_get_with_too_many_args_returns_error() {
        let mut vm = Vm::default();
        assert_eq!(
            vm.eval_str("(struct-get 1 2 3)").unwrap_err(),
            VmError::ArityError {
                function: "struct-get".into(),
                expected: 2,
                actual: 3
            }
        );
    }

    #[test]
    fn struct_get_with_non_struct_returns_error() {
        let mut vm = Vm::default();
        let src = "(struct-get 1 'field)";
        assert_eq!(
            vm.eval_str(src).unwrap_err(),
            VmError::TypeError {
                src: Some(Span::new(0, 21).with_src(src.into())),
                context: "struct-get arg(idx=0)",
                expected: UnsafeVal::STRUCT_TYPE_NAME,
                actual: "int",
                value: "1".into()
            }
        );
    }

    #[test]
    fn struct_get_with_non_string_returns_error() {
        let mut vm = Vm::default();
        let src = "(struct-get (struct) 1)";
        assert_eq!(
            vm.eval_str(src).unwrap_err(),
            VmError::TypeError {
                src: Some(Span::new(0, 23).with_src(src.into())),
                context: "struct-get arg(idx=1)",
                expected: UnsafeVal::SYMBOL_TYPE_NAME,
                actual: "int",
                value: "1".into()
            }
        );
    }

    #[test]
    fn struct_set_sets_existing_field() {
        let mut vm = Vm::default();
        vm.eval_str("(define x (struct 'field \"original\"))")
            .unwrap();
        assert_eq!(
            vm.eval_str("(struct-get x 'field)")
                .unwrap()
                .try_str()
                .unwrap(),
            "original"
        );
        vm.eval_str("(struct-set! x 'field \"new\")").unwrap();
        assert_eq!(
            vm.eval_str("(struct-get x 'field)")
                .unwrap()
                .try_str()
                .unwrap(),
            "new"
        );
    }

    #[test]
    fn struct_set_sets_new_field() {
        let mut vm = Vm::default();
        vm.eval_str("(define x (struct 'field \"original\"))")
            .unwrap();
        vm.eval_str("(struct-set! x 'field2 \"new\")").unwrap();
        assert!(vm
            .eval_str("(= x (struct 'field \"original\" 'field2 \"new\"))")
            .unwrap()
            .try_bool()
            .unwrap(),);
    }

    #[test]
    fn struct_set_with_non_struct_returns_error() {
        let mut vm = Vm::default();
        let src = "(struct-set! 1 'field 3)";
        assert_eq!(
            vm.eval_str(src).unwrap_err(),
            VmError::TypeError {
                src: Some(Span::new(0, 24).with_src(src.into())),
                context: "struct-set! arg(idx=0)",
                expected: UnsafeVal::STRUCT_TYPE_NAME,
                actual: UnsafeVal::INT_TYPE_NAME,
                value: "1".into(),
            },
        );
    }

    #[test]
    fn struct_set_with_non_string_field_returns_error() {
        let mut vm = Vm::default();
        vm.eval_str("(define x (struct))").unwrap();
        let src = "(struct-set! x 2 3)";
        assert_eq!(
            vm.eval_str(src).unwrap_err(),
            VmError::TypeError {
                src: Some(Span::new(0, 19).with_src(src.into())),
                context: "struct-set! arg(idx=1)",
                expected: UnsafeVal::SYMBOL_TYPE_NAME,
                actual: UnsafeVal::INT_TYPE_NAME,
                value: "2".into(),
            },
        );
    }

    #[test]
    fn struct_set_with_too_many_args_returns_error() {
        let mut vm = Vm::default();
        vm.eval_str("(define x (struct))").unwrap();
        assert_eq!(
            vm.eval_str("(struct-set! x 'field 2 3)").unwrap_err(),
            VmError::ArityError {
                function: "struct-set!".into(),
                expected: 3,
                actual: 4
            },
        );
    }
}
