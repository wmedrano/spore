use crate::{
    error::{VmError, VmResult},
    val::{
        native_function::{NativeFunction, NativeFunctionContext},
        InternalVal,
    },
    Vm,
};

pub const BUILTINS: &[(&str, NativeFunction)] = &[
    ("+", add),
    ("<", less),
    ("string-join", string_join),
    ("list", list),
    ("working-directory", working_directory),
];

pub fn add(ctx: NativeFunctionContext) -> VmResult<InternalVal> {
    let mut int_sum: i64 = 0;
    let mut float_sum: f64 = 0.0;
    let mut has_float = false;
    for arg in ctx.args() {
        match arg {
            InternalVal::Int(x) => int_sum += *x,
            InternalVal::Float(x) => {
                float_sum += *x;
                has_float = true;
            }
            v => {
                return Err(VmError::TypeError {
                    context: "+",
                    expected: InternalVal::INT_TYPE_NAME,
                    actual: v.type_name(),
                    value: v.format_quoted(ctx.vm()).to_string(),
                })
            }
        }
    }
    if has_float {
        Ok(InternalVal::Float(float_sum + int_sum as f64))
    } else {
        Ok(InternalVal::Int(int_sum))
    }
}

fn less_two_impl(vm: &Vm, a: &InternalVal, b: &InternalVal) -> VmResult<bool> {
    match (a, b) {
        (InternalVal::Int(a), InternalVal::Int(b)) => Ok(*a < *b),
        (InternalVal::Float(a), InternalVal::Float(b)) => Ok(*a < *b),
        (InternalVal::Float(a), InternalVal::Int(b)) => Ok(*a < (*b as f64)),
        (InternalVal::Int(a), InternalVal::Float(b)) => Ok((*a as f64) < *b),
        (a, InternalVal::Int(_)) | (a, InternalVal::Float(_)) => Err(VmError::TypeError {
            context: "<",
            expected: "int or float",
            actual: a.type_name(),
            value: a.format_quoted(vm).to_string(),
        }),
        (_, b) => Err(VmError::TypeError {
            context: "<",
            expected: "int or float",
            actual: b.type_name(),
            value: b.format_quoted(vm).to_string(),
        }),
    }
}

pub fn less_impl(vm: &Vm, args: &[InternalVal]) -> VmResult<InternalVal> {
    match args {
        [] | [_] => Ok(InternalVal::Bool(true)),
        [a, b] => Ok(InternalVal::Bool(less_two_impl(vm, a, b)?)),
        [a, b, ..] => match less_two_impl(vm, a, b)? {
            true => less_impl(vm, &args[1..]),
            false => Ok(InternalVal::Bool(false)),
        },
    }
}

pub fn less(ctx: NativeFunctionContext) -> VmResult<InternalVal> {
    less_impl(ctx.vm(), ctx.args())
}

pub fn string_join(mut ctx: NativeFunctionContext) -> VmResult<InternalVal> {
    let args = ctx.args();
    let (strings, separator) = match args {
        [] => {
            return Err(VmError::ArityError {
                function: "string-join".to_string(),
                expected: 1,
                actual: 0,
            })
        }
        [InternalVal::List(list)] => (*list, ""),
        [v] => {
            return Err(VmError::TypeError {
                context: "string-join arg(idx=0)",
                expected: InternalVal::LIST_TYPE_NAME,
                actual: v.type_name(),
                value: v.format_quoted(ctx.vm()).to_string(),
            });
        }
        [InternalVal::List(list), InternalVal::String(string)] => {
            (*list, ctx.vm().val_store.get_str(*string))
        }
        [_, v] => {
            return Err(VmError::TypeError {
                context: "string-join arg(idx=1)",
                expected: InternalVal::STRING_TYPE_NAME,
                actual: v.type_name(),
                value: v.format_quoted(ctx.vm()).to_string(),
            });
        }
        _ => {
            return Err(VmError::ArityError {
                function: "string-join".to_string(),
                expected: 2,
                actual: args.len(),
            })
        }
    };
    let mut result = String::new();
    for (idx, string_id) in ctx.vm().val_store.get_list(strings).iter().enumerate() {
        if idx > 0 {
            result.push_str(separator);
        }
        match string_id {
            InternalVal::String(string_id) => {
                result.push_str(ctx.vm().val_store.get_str(*string_id));
            }
            v => {
                return Err(VmError::TypeError {
                    context: "string-join arg(idx=0)",
                    expected: InternalVal::STRING_TYPE_NAME,
                    actual: v.type_name(),
                    value: v.format_quoted(ctx.vm()).to_string(),
                })
            }
        };
    }
    // Unsafe OK: Value is returned immediately so vm does not have chance to run garbage
    // collection.
    Ok(unsafe { ctx.new_string(result) })
}

pub fn list(mut ctx: NativeFunctionContext) -> VmResult<InternalVal> {
    let list = ctx.args().to_vec();
    // Unsafe OK: Value is returned immediately so vm does not have chance to run garbage
    // collection.
    Ok(unsafe { ctx.new_list(list) })
}

pub fn working_directory(mut ctx: NativeFunctionContext) -> VmResult<InternalVal> {
    let arg_len = ctx.arg_len();
    if arg_len != 0 {
        return Err(VmError::ArityError {
            function: "working-directory".to_string(),
            expected: 0,
            actual: arg_len,
        });
    }
    let working_directory = match std::env::current_dir() {
        Ok(path) => path.to_string_lossy().to_string(),
        // Untested OK: It is hard to create a working directory error and is not common.
        Err(err) => return Err(VmError::CustomError(err.to_string())),
    };
    // Unsafe OK: Value is returned immediately so vm does not have chance to run garbage
    // collection.
    Ok(unsafe { ctx.new_string(working_directory) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_with_no_args_is_int_0() {
        let mut vm = Vm::default();
        let got = vm.eval_str("(+)").unwrap();
        assert_eq!(got.as_int(), Some(0));
        assert_eq!(got.as_float(), None);
    }

    #[test]
    fn add_with_nonnumber_is_type_error() {
        let mut vm = Vm::default();
        let got = vm.eval_str("(+ 1 2 \"fish\")").unwrap_err();
        assert_eq!(
            got,
            VmError::TypeError {
                context: "+",
                expected: InternalVal::INT_TYPE_NAME,
                actual: InternalVal::STRING_TYPE_NAME,
                value: "\"fish\"".to_string(),
            }
        );
    }

    #[test]
    fn add_ints_produces_int() {
        let mut vm = Vm::default();
        let got = vm.eval_str("(+ 1 2 3)").unwrap();
        assert_eq!(got.as_int(), Some(6));
        assert_eq!(got.as_float(), None);
    }

    #[test]
    fn add_floats_produces_floats() {
        let mut vm = Vm::default();
        let got = vm.eval_str("(+ 1.0 2.0 3.0)").unwrap();
        assert_eq!(got.as_float(), Some(6.0));
        assert_eq!(got.as_int(), None);
    }

    #[test]
    fn add_ints_and_floats_produces_floats() {
        let mut vm = Vm::default();
        let got = vm.eval_str("(+ 1 2.0 3)").unwrap();
        assert_eq!(got.as_float(), Some(6.0));
        assert_eq!(got.as_int(), None);
    }

    #[test]
    fn less_with_no_args_is_true() {
        let mut vm = Vm::default();
        let got = vm.eval_str("(<)").unwrap();
        assert_eq!(got.as_bool(), Some(true));
    }

    #[test]
    fn less_with_single_arg_is_true() {
        let mut vm = Vm::default();
        let got = vm.eval_str("(< 1)").unwrap();
        assert_eq!(got.as_bool(), Some(true));
    }

    #[test]
    fn less_with_increasing_ordered_args_is_true() {
        let mut vm = Vm::default();
        let got = vm.eval_str("(< -1 0 1 1.2 1.8 2)").unwrap();
        assert_eq!(got.as_bool(), Some(true));
    }

    #[test]
    fn less_with_unordered_args_is_false() {
        let mut vm = Vm::default();
        let got = vm.eval_str("(< -1 0 -0.1 1.2 2)").unwrap();
        assert_eq!(got.as_bool(), Some(false));
    }

    #[test]
    fn less_with_nonumber_args_is_type_error() {
        let mut vm = Vm::default();
        let got = vm.eval_str("(< \"blue\" 2)").unwrap_err();
        assert_eq!(
            got,
            VmError::TypeError {
                context: "<",
                expected: "int or float",
                actual: InternalVal::STRING_TYPE_NAME,
                value: "\"blue\"".to_string(),
            }
        );
        let got = vm.eval_str("(< \"blue\" 2.0)").unwrap_err();
        assert_eq!(
            got,
            VmError::TypeError {
                context: "<",
                expected: "int or float",
                actual: InternalVal::STRING_TYPE_NAME,
                value: "\"blue\"".to_string(),
            }
        );
        let got = vm.eval_str("(< -1 \"fish\" 2)").unwrap_err();
        assert_eq!(
            got,
            VmError::TypeError {
                context: "<",
                expected: "int or float",
                actual: InternalVal::STRING_TYPE_NAME,
                value: "\"fish\"".to_string(),
            }
        );
    }

    #[test]
    fn string_join_on_empty_list_is_empty() {
        let mut vm = Vm::default();
        let got = vm.eval_str("(string-join (list))").unwrap();
        assert_eq!(got.as_str(), Some(""));
    }

    #[test]
    fn string_join_with_wrong_number_of_args_is_arity_error() {
        let mut vm = Vm::default();
        assert_eq!(
            vm.eval_str("(string-join)").unwrap_err(),
            VmError::ArityError {
                function: "string-join".to_string(),
                expected: 1,
                actual: 0,
            },
        );
        assert_eq!(
            vm.eval_str("(string-join 1 2 3)").unwrap_err(),
            VmError::ArityError {
                function: "string-join".to_string(),
                expected: 2,
                actual: 3,
            },
        );
    }

    #[test]
    fn string_join_with_wrong_type_args_is_type_error() {
        let mut vm = Vm::default();
        assert_eq!(
            vm.eval_str("(string-join 2)").unwrap_err(),
            VmError::TypeError {
                context: "string-join arg(idx=0)",
                expected: InternalVal::LIST_TYPE_NAME,
                actual: InternalVal::INT_TYPE_NAME,
                value: "2".to_string(),
            },
        );
        assert_eq!(
            vm.eval_str("(string-join (list \"ok string\" 42))")
                .unwrap_err(),
            VmError::TypeError {
                context: "string-join arg(idx=0)",
                expected: InternalVal::STRING_TYPE_NAME,
                actual: InternalVal::INT_TYPE_NAME,
                value: "42".to_string(),
            },
        );
        assert_eq!(
            vm.eval_str("(string-join (list) 3)").unwrap_err(),
            VmError::TypeError {
                context: "string-join arg(idx=1)",
                expected: InternalVal::STRING_TYPE_NAME,
                actual: InternalVal::INT_TYPE_NAME,
                value: "3".to_string(),
            },
        );
    }

    #[test]
    fn string_join_with_no_separator_concatenates() {
        let mut vm = Vm::default();
        let got = vm.eval_str("(string-join (list \"one\" \"two\"))").unwrap();
        assert_eq!(got.as_str(), Some("onetwo"));
    }

    #[test]
    fn string_join_with_custom_separator_concatenates_with_separator() {
        let mut vm = Vm::default();
        let got = vm
            .eval_str("(string-join (list \"one\" \"two\") \" fish \")")
            .unwrap();
        assert_eq!(got.as_str(), Some("one fish two"));
    }

    #[test]
    fn working_directory_with_args_produces_arity_error() {
        let mut vm = Vm::default();
        assert_eq!(
            vm.eval_str("(working-directory 1)").unwrap_err(),
            VmError::ArityError {
                function: "working-directory".to_string(),
                expected: 0,
                actual: 1
            }
        );
    }

    #[test]
    fn working_directory_produces_working_directory_path() {
        let mut vm = Vm::default();
        let working_directory = std::env::current_dir()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        assert_ne!(working_directory, "");
        let got = vm.eval_str("(working-directory)").unwrap();
        assert_eq!(got.as_str(), Some(working_directory.as_str()));
    }
}
