use compact_str::CompactString;

use crate::{
    error::{VmError, VmResult},
    val::{NativeFunctionContext, UnsafeVal, Val, ValBuilder},
    Vm,
};

pub fn string_length<'a>(ctx: NativeFunctionContext, args: &[Val<'a>]) -> VmResult<ValBuilder<'a>> {
    match args {
        [v] => v
            .try_str(ctx.vm())
            .map_err(|v| VmError::TypeError {
                src: None,
                context: "string-length",
                expected: UnsafeVal::STRING_TYPE_NAME,
                actual: v.type_name(),
                value: v.formatted(ctx.vm()).to_string(),
            })
            .map(|s| ValBuilder::new((s.len() as i64).into())),
        _ => Err(VmError::ArityError {
            function: "string-length".into(),
            expected: 1,
            actual: args.len(),
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

pub fn string_split<'a>(
    mut ctx: NativeFunctionContext<'a>,
    args: &[Val],
) -> VmResult<ValBuilder<'a>> {
    let vm: *mut Vm = unsafe { ctx.vm_mut() };
    match args {
        [maybe_string] => {
            let string = maybe_string
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
        [maybe_string, maybe_separator] => {
            let string = maybe_string
                .try_str(unsafe { &*vm })
                .map_err(|v| VmError::TypeError {
                    src: None,
                    context: "string-split arg(idx = 0)",
                    expected: UnsafeVal::STRING_TYPE_NAME,
                    actual: v.type_name(),
                    value: v.format_quoted(ctx.vm()).to_string(),
                })?;
            let separator =
                maybe_separator
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
        _ => Err(VmError::ArityError {
            function: "string-split".into(),
            expected: if args.is_empty() { 1 } else { 2 },
            actual: args.len(),
        }),
    }
}

pub fn string_join<'a>(ctx: NativeFunctionContext<'a>, args: &[Val]) -> VmResult<ValBuilder<'a>> {
    let (list, separator) = match args {
        [maybe_list] => {
            let list = maybe_list
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
        [maybe_list, maybe_separator] => {
            let list = maybe_list
                .try_list(ctx.vm())
                .map_err(|v| VmError::TypeError {
                    src: None,
                    context: "string-join arg(idx=0)",
                    expected: UnsafeVal::LIST_TYPE_NAME,
                    actual: v.type_name(),
                    value: v.format_quoted(ctx.vm()).to_string(),
                })?;
            let separator = maybe_separator
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
        [] => {
            return Err(VmError::ArityError {
                function: "string-join".into(),
                expected: 1,
                actual: 0,
            })
        }
        args => {
            return Err(VmError::ArityError {
                function: "string-join".into(),
                expected: 2,
                actual: args.len(),
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
                context: "string-split arg(idx = 0)".into(),
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
                context: "string-split arg(idx = 1)".into(),
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
                context: "string-split arg(idx = 0)".into(),
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
