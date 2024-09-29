use std::collections::HashMap;

use compact_str::ToCompactString;

use crate::{
    error::{VmError, VmResult},
    val::{NativeFunctionContext, UnsafeVal, Val, ValBuilder},
};

pub fn strct<'a>(ctx: NativeFunctionContext<'a>, args: &[Val<'a>]) -> VmResult<ValBuilder<'a>> {
    let mut args_iter = args.iter();
    let mut strct = HashMap::with_capacity(args.len() / 2);
    if args.len() % 2 != 0 {
        return Err(VmError::ArityError {
            function: "struct needs an even amount of args, ".into(),
            expected: args.len() + 1,
            actual: args.len(),
        });
    }
    while let Some(field) = args_iter.next() {
        let field = field.try_str(ctx.vm()).unwrap().to_compact_string();
        let val = args_iter.next().unwrap();
        strct.insert(field, unsafe { val.as_unsafe_val() });
    }
    Ok(unsafe { ctx.new_struct(strct) })
}

pub fn struct_get<'a>(ctx: NativeFunctionContext<'a>, args: &[Val]) -> VmResult<ValBuilder<'a>> {
    match args {
        [maybe_struct, maybe_string] => {
            let field = maybe_string
                .try_str(ctx.vm())
                .map_err(|v| VmError::TypeError {
                    context: "struct-get arg(idx=1)",
                    expected: UnsafeVal::STRING_TYPE_NAME,
                    actual: v.type_name(),
                    value: v.format_quoted(ctx.vm()).to_string(),
                })?;
            // Unsafe OK: The returned val is a reference to a valid value.
            let v = unsafe {
                maybe_struct
                    .try_struct_get(ctx.vm(), field)
                    .map_err(|v| VmError::TypeError {
                        context: "struct-get arg(idx=0)",
                        expected: UnsafeVal::STRUCT_TYPE_NAME,
                        actual: v.type_name(),
                        value: v.format_quoted(ctx.vm()).to_string(),
                    })?
                    .unwrap_or(Val::new_void())
                    .as_unsafe_val()
            };
            Ok(unsafe { ctx.with_unsafe_val(v) })
        }
        args => Err(VmError::ArityError {
            function: "struct-get".into(),
            expected: 2,
            actual: args.len(),
        }),
    }
}

pub fn struct_set<'a>(
    mut ctx: NativeFunctionContext<'a>,
    args: &[Val<'a>],
) -> VmResult<ValBuilder<'a>> {
    match args {
        [maybe_struct, maybe_string, val] => {
            let field = maybe_string
                .try_str(ctx.vm())
                .map_err(|v| VmError::TypeError {
                    context: "struct-set! arg(idx=1)",
                    expected: UnsafeVal::STRING_TYPE_NAME,
                    actual: v.type_name(),
                    value: v.format_quoted(ctx.vm()).to_string(),
                })?
                .to_compact_string();
            // Unsafe OK: The returned val is a reference to a valid value.
            let strct = match unsafe { maybe_struct.try_unsafe_struct_mut(ctx.vm_mut()) } {
                Ok(v) => v,
                Err(v) => {
                    return Err(VmError::TypeError {
                        context: "struct-set! arg(idx=0)",
                        expected: UnsafeVal::STRUCT_TYPE_NAME,
                        actual: v.type_name(),
                        value: v.format_quoted(ctx.vm()).to_string(),
                    });
                }
            };
            strct.insert(field, unsafe { val.as_unsafe_val() });
            Ok(ValBuilder::new(().into()))
        }
        args => Err(VmError::ArityError {
            function: "struct-set!".into(),
            expected: 3,
            actual: args.len(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use crate::Vm;

    use super::*;

    #[test]
    fn struct_with_no_args_produces_empty_struct() {
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
            vm.eval_str("(struct \"field\")").unwrap_err(),
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
            .eval_str("(struct-get (struct \"field\" 1.0) \"field\")")
            .unwrap();
        assert_eq!(got.try_float().unwrap(), 1.0);
    }

    #[test]
    fn struct_get_with_field_that_does_not_exist_returns_void() {
        let mut vm = Vm::default();
        let got = vm
            .eval_str("(struct-get (struct \"field\" 1) \"not-field\")")
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
        assert_eq!(
            vm.eval_str("(struct-get 1 \"field\")").unwrap_err(),
            VmError::TypeError {
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
        assert_eq!(
            vm.eval_str("(struct-get (struct) 1)").unwrap_err(),
            VmError::TypeError {
                context: "struct-get arg(idx=1)",
                expected: UnsafeVal::STRING_TYPE_NAME,
                actual: "int",
                value: "1".into()
            }
        );
    }

    #[test]
    fn struct_set_sets_existing_field() {
        let mut vm = Vm::default();
        vm.eval_str("(define x (struct \"field\" \"original\"))")
            .unwrap();
        assert_eq!(
            vm.eval_str("(struct-get x \"field\")")
                .unwrap()
                .try_str()
                .unwrap(),
            "original"
        );
        vm.eval_str("(struct-set! x \"field\" \"new\")").unwrap();
        assert_eq!(
            vm.eval_str("(struct-get x \"field\")")
                .unwrap()
                .try_str()
                .unwrap(),
            "new"
        );
    }

    #[test]
    fn struct_set_sets_new_field() {
        let mut vm = Vm::default();
        vm.eval_str("(define x (struct \"field\" \"original\"))")
            .unwrap();
        vm.eval_str("(struct-set! x \"field2\" \"new\")").unwrap();
        assert!(vm
            .eval_str("(= x (struct \"field\" \"original\" \"field2\" \"new\"))")
            .unwrap()
            .try_bool()
            .unwrap(),);
    }

    #[test]
    fn struct_set_with_non_struct_returns_error() {
        let mut vm = Vm::default();
        assert_eq!(
            vm.eval_str("(struct-set! 1 \"field\" 3)").unwrap_err(),
            VmError::TypeError {
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
        assert_eq!(
            vm.eval_str("(struct-set! x 2 3)").unwrap_err(),
            VmError::TypeError {
                context: "struct-set! arg(idx=1)",
                expected: UnsafeVal::STRING_TYPE_NAME,
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
            vm.eval_str("(struct-set! x \"field\" 2 3)").unwrap_err(),
            VmError::ArityError {
                function: "struct-set!".into(),
                expected: 3,
                actual: 4
            },
        );
    }
}
