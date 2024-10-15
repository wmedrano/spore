use compact_str::CompactString;

use crate::{
    error::{VmError, VmResult},
    val::{NativeFunctionContext, UnsafeVal, ValBuilder},
    Vm,
};

pub fn string_length<'a>(ctx: NativeFunctionContext) -> VmResult<ValBuilder<'a>> {
    match ctx.arg_count() {
        1 => ctx
            .arg(0)
            .unwrap()
            .try_str(ctx.vm())
            .map_err(|v| VmError::TypeError {
                src: None,
                context: "string-length",
                expected: UnsafeVal::STRING_TYPE_NAME,
                actual: v.type_name(),
                value: v.formatted(ctx.vm()).to_string(),
            })
            .map(|s| ValBuilder::new((s.len() as i64).into())),
        n => Err(VmError::ArityError {
            function: "string-length".into(),
            expected: 1,
            actual: n,
        }),
    }
}

fn string_split_impl<'a, 'b>(
    mut ctx: NativeFunctionContext<'a>,
    strs: impl Iterator<Item = &'b str>,
) -> ValBuilder<'a> {
    let vm: *mut Vm = unsafe { ctx.vm_mut() };
    let ret: Vec<UnsafeVal> = strs
        .map(|l| (unsafe { &mut *vm }).objects.insert_string(l.into()))
        .map(UnsafeVal::String)
        .collect();
    let list = UnsafeVal::List(unsafe { ctx.vm_mut().objects.insert_list(ret) });
    unsafe { ctx.with_unsafe_val(list) }
}

pub fn string_split(mut ctx: NativeFunctionContext<'_>) -> VmResult<ValBuilder<'_>> {
    let vm: *mut Vm = unsafe { ctx.vm_mut() };
    match ctx.arg_count() {
        1 => {
            let string =
                ctx.arg(0)
                    .unwrap()
                    .try_str(unsafe { &*vm })
                    .map_err(|v| VmError::TypeError {
                        src: None,
                        context: "string-split arg(idx = 0)",
                        expected: UnsafeVal::STRING_TYPE_NAME,
                        actual: v.type_name(),
                        value: v.format_quoted(ctx.vm()).to_string(),
                    })?;
            Ok(string_split_impl(ctx, string.split('\n')))
        }
        2 => {
            let string =
                ctx.arg(0)
                    .unwrap()
                    .try_str(unsafe { &*vm })
                    .map_err(|v| VmError::TypeError {
                        src: None,
                        context: "string-split arg(idx = 0)",
                        expected: UnsafeVal::STRING_TYPE_NAME,
                        actual: v.type_name(),
                        value: v.format_quoted(ctx.vm()).to_string(),
                    })?;
            let separator =
                ctx.arg(1)
                    .unwrap()
                    .try_str(unsafe { &*vm })
                    .map_err(|v| VmError::TypeError {
                        src: None,
                        context: "string-split arg(idx = 1)",
                        expected: UnsafeVal::STRING_TYPE_NAME,
                        actual: v.type_name(),
                        value: v.format_quoted(ctx.vm()).to_string(),
                    })?;
            Ok(string_split_impl(ctx, string.split(separator)))
        }
        n => Err(VmError::ArityError {
            function: "string-split".into(),
            expected: if n == 0 { 1 } else { 2 },
            actual: n,
        }),
    }
}

pub fn string_join(ctx: NativeFunctionContext<'_>) -> VmResult<ValBuilder<'_>> {
    let (list, separator) = match ctx.arg_count() {
        1 => {
            let list = ctx
                .arg(0)
                .unwrap()
                .try_list(ctx.vm())
                .map_err(|v| VmError::TypeError {
                    src: None,
                    context: "string-join arg(idx=0)",
                    expected: UnsafeVal::LIST_TYPE_NAME,
                    actual: v.type_name(),
                    value: v.format_quoted(ctx.vm()).to_string(),
                })?;
            (list, "")
        }
        2 => {
            let list = ctx
                .arg(0)
                .unwrap()
                .try_list(ctx.vm())
                .map_err(|v| VmError::TypeError {
                    src: None,
                    context: "string-join arg(idx=0)",
                    expected: UnsafeVal::LIST_TYPE_NAME,
                    actual: v.type_name(),
                    value: v.format_quoted(ctx.vm()).to_string(),
                })?;
            let separator =
                ctx.arg(1)
                    .unwrap()
                    .try_str(ctx.vm())
                    .map_err(|v| VmError::TypeError {
                        src: None,
                        context: "string-join arg(idx=1)",
                        expected: UnsafeVal::STRING_TYPE_NAME,
                        actual: v.type_name(),
                        value: v.format_quoted(ctx.vm()).to_string(),
                    })?;
            (list, separator)
        }
        0 => {
            return Err(VmError::ArityError {
                function: "string-join".into(),
                expected: 1,
                actual: 0,
            })
        }
        n => {
            return Err(VmError::ArityError {
                function: "string-join".into(),
                expected: 2,
                actual: n,
            })
        }
    };
    let mut result = CompactString::default();
    for (idx, list_element) in list.iter().enumerate() {
        if idx > 0 {
            result.push_str(separator);
        }
        result.push_str(
            list_element
                .try_str(ctx.vm())
                .map_err(|v| VmError::TypeError {
                    src: None,
                    context: "string-join arg(idx=0) list subelement",
                    expected: UnsafeVal::STRING_TYPE_NAME,
                    actual: v.type_name(),
                    value: v.format_quoted(ctx.vm()).to_string(),
                })?,
        );
    }
    Ok(ctx.new_string(result))
}

#[cfg(test)]
mod tests {
    use crate::{parser::span::Span, Vm};

    use super::*;

    #[test]
    fn string_length_with_empty_string_is_zero() {
        let mut vm = Vm::default();
        assert_eq!(
            vm.eval_str("(string-length \"\")")
                .unwrap()
                .try_int()
                .unwrap(),
            0
        );
    }

    #[test]
    fn string_length_gives_string_length() {
        let mut vm = Vm::default();
        assert_eq!(
            vm.eval_str("(string-length \"1234\")")
                .unwrap()
                .try_int()
                .unwrap(),
            4
        );
    }

    #[test]
    fn string_length_with_wrong_args_returns_error() {
        let mut vm = Vm::default();
        assert_eq!(
            vm.eval_str("(string-length)").unwrap_err(),
            VmError::ArityError {
                function: "string-length".into(),
                expected: 1,
                actual: 0
            }
        );
        assert_eq!(
            vm.eval_str("(string-length \"\" \"\")").unwrap_err(),
            VmError::ArityError {
                function: "string-length".into(),
                expected: 1,
                actual: 2
            }
        );
        let src = "(string-length 0)";
        assert_eq!(
            vm.eval_str(src).unwrap_err(),
            VmError::TypeError {
                src: Some(Span::new(0, 17).with_src(src.into())),
                context: "string-length",
                expected: UnsafeVal::STRING_TYPE_NAME,
                actual: UnsafeVal::INT_TYPE_NAME,
                value: "0".into(),
            }
        );
    }

    #[test]
    fn string_split_splits_by_line() {
        let mut vm = Vm::default();
        assert_eq!(
            vm.eval_str("(string-split \"one\ntwo\n\")")
                .unwrap()
                .to_string(),
            "(\"one\" \"two\" \"\")"
        );
    }

    #[test]
    fn string_split_with_custom_separator_splits_by_separator() {
        let mut vm = Vm::default();
        assert_eq!(
            vm.eval_str("(string-split \"one, two, three\" \", \")")
                .unwrap()
                .to_string(),
            "(\"one\" \"two\" \"three\")"
        );
    }

    #[test]
    fn string_split_with_wrong_args_returns_error() {
        let mut vm = Vm::default();
        let src = "(string-split)";
        assert_eq!(
            vm.eval_str(src).unwrap_err(),
            VmError::ArityError {
                function: "string-split".into(),
                expected: 1,
                actual: 0
            }
        );

        let src = "(string-split \"\" \"\" \"\")";
        assert_eq!(
            vm.eval_str(src).unwrap_err(),
            VmError::ArityError {
                function: "string-split".into(),
                expected: 2,
                actual: 3
            }
        );

        let src = "(string-split 1)";
        assert_eq!(
            vm.eval_str(src).unwrap_err(),
            VmError::TypeError {
                context: "string-split arg(idx = 0)",
                src: Some(Span::new(0, 16).with_src(src.into())),
                expected: UnsafeVal::STRING_TYPE_NAME,
                actual: UnsafeVal::INT_TYPE_NAME,
                value: "1".to_string(),
            }
        );

        let src = "(string-split \"\" 1)";
        assert_eq!(
            vm.eval_str(src).unwrap_err(),
            VmError::TypeError {
                context: "string-split arg(idx = 1)",
                src: Some(Span::new(0, 19).with_src(src.into())),
                expected: UnsafeVal::STRING_TYPE_NAME,
                actual: UnsafeVal::INT_TYPE_NAME,
                value: "1".to_string(),
            }
        );

        let src = "(string-split 1 \"\")";
        assert_eq!(
            vm.eval_str(src).unwrap_err(),
            VmError::TypeError {
                context: "string-split arg(idx = 0)",
                src: Some(Span::new(0, 19).with_src(src.into())),
                expected: UnsafeVal::STRING_TYPE_NAME,
                actual: UnsafeVal::INT_TYPE_NAME,
                value: "1".to_string(),
            }
        );
    }

    #[test]
    fn string_join_on_empty_list_is_empty() {
        let mut vm = Vm::default();
        let got = vm.eval_str("(string-join (list))").unwrap();
        assert_eq!(got.try_str().unwrap(), "");
    }

    #[test]
    fn string_join_with_wrong_number_of_args_is_arity_error() {
        let mut vm = Vm::default();
        assert_eq!(
            vm.eval_str("(string-join)").unwrap_err(),
            VmError::ArityError {
                function: "string-join".into(),
                expected: 1,
                actual: 0,
            },
        );
        assert_eq!(
            vm.eval_str("(string-join (list) \"\" 3)").unwrap_err(),
            VmError::ArityError {
                function: "string-join".into(),
                expected: 2,
                actual: 3,
            },
        );
    }

    #[test]
    fn string_join_with_wrong_type_args_is_type_error() {
        let mut vm = Vm::default();
        let src = "(string-join 2)";
        assert_eq!(
            vm.eval_str(src).unwrap_err(),
            VmError::TypeError {
                src: Some(Span::new(0, 15).with_src(src.into())),
                context: "string-join arg(idx=0)",
                expected: UnsafeVal::LIST_TYPE_NAME,
                actual: UnsafeVal::INT_TYPE_NAME,
                value: "2".to_string(),
            },
        );
        let src = "(string-join 3 \",\")";
        assert_eq!(
            vm.eval_str(src).unwrap_err(),
            VmError::TypeError {
                src: Some(Span::new(0, 19).with_src(src.into())),
                context: "string-join arg(idx=0)",
                expected: UnsafeVal::LIST_TYPE_NAME,
                actual: UnsafeVal::INT_TYPE_NAME,
                value: "3".to_string(),
            },
        );
        let src = "(string-join (list \"ok string\" 42))";
        assert_eq!(
            vm.eval_str(src).unwrap_err(),
            VmError::TypeError {
                src: Some(Span::new(0, 35).with_src(src.into())),
                context: "string-join arg(idx=0) list subelement",
                expected: UnsafeVal::STRING_TYPE_NAME,
                actual: UnsafeVal::INT_TYPE_NAME,
                value: "42".to_string(),
            },
        );
        let src = "(string-join (list) 3)";
        assert_eq!(
            vm.eval_str(src).unwrap_err(),
            VmError::TypeError {
                src: Some(Span::new(0, 22).with_src(src.into())),
                context: "string-join arg(idx=1)",
                expected: UnsafeVal::STRING_TYPE_NAME,
                actual: UnsafeVal::INT_TYPE_NAME,
                value: "3".to_string(),
            },
        );
    }

    #[test]
    fn string_join_with_no_separator_concatenates() {
        let mut vm = Vm::default();
        let got = vm.eval_str("(string-join (list \"one\" \"two\"))").unwrap();
        assert_eq!(got.try_str().unwrap(), "onetwo");
    }

    #[test]
    fn string_join_with_custom_separator_concatenates_with_separator() {
        let mut vm = Vm::default();
        let got = vm
            .eval_str("(string-join (list \"one\" \"two\") \" fish \")")
            .unwrap();
        assert_eq!(got.try_str().unwrap(), "one fish two");
    }
}
