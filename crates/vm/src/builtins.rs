use compact_str::CompactString;

use crate::{
    error::{VmError, VmResult},
    val::{NativeFunction, NativeFunctionContext, UnsafeVal, ValBuilder},
    Vm,
};

pub const BUILTINS: &[(&str, NativeFunction)] = &[
    ("=", equal),
    ("+", add),
    ("<", less),
    ("truthy?", truthy_p),
    ("not", not),
    ("string-join", string_join),
    ("new-box", new_box),
    ("set-box!", set_box),
    ("unbox", unbox),
    ("list", list),
    ("working-directory", working_directory),
];

pub fn equal(ctx: NativeFunctionContext) -> VmResult<ValBuilder> {
    let args = ctx.args();
    match args {
        [a, b] => Ok(ValBuilder::new_bool(equal_impl(ctx.vm(), *a, *b))),
        _ => Err(VmError::ArityError {
            function: "=".into(),
            expected: 2,
            actual: args.len(),
        }),
    }
}

pub fn equal_impl(vm: &Vm, a: UnsafeVal, b: UnsafeVal) -> bool {
    use crate::val::UnsafeVal::*;
    match (a, b) {
        (Void, Void) => true,
        (Bool(a), Bool(b)) => a == b,
        (Int(a), Int(b)) => a == b,
        (Float(a), Float(b)) => a == b,
        (String(a), String(b)) => vm.objects.get_str(a) == vm.objects.get_str(b),
        (List(a), List(b)) => {
            let a = vm.objects.get_list(a);
            let b = vm.objects.get_list(b);
            if a.len() != b.len() {
                return false;
            }
            a.iter().zip(b.iter()).all(|(a, b)| equal_impl(vm, *a, *b))
        }
        (ByteCodeFunction(a), ByteCodeFunction(b)) => a == b,
        (NativeFunction(a), NativeFunction(b)) => a == b,
        _ => false,
    }
}

pub fn add<'a>(ctx: NativeFunctionContext) -> VmResult<ValBuilder<'a>> {
    let mut int_sum: i64 = 0;
    let mut float_sum: f64 = 0.0;
    let mut has_float = false;
    for arg in ctx.args() {
        match arg {
            UnsafeVal::Int(x) => int_sum += x,
            UnsafeVal::Float(x) => {
                float_sum += x;
                has_float = true;
            }
            _ => {
                return Err(VmError::TypeError {
                    context: "+",
                    expected: UnsafeVal::INT_TYPE_NAME,
                    actual: arg.type_name(),
                    value: arg.format_quoted(ctx.vm()).to_string(),
                })
            }
        }
    }
    if has_float {
        Ok(ValBuilder::new_float(float_sum + int_sum as f64))
    } else {
        Ok(ValBuilder::new_int(int_sum))
    }
}

fn less_two_impl(vm: &Vm, a: UnsafeVal, b: UnsafeVal) -> VmResult<bool> {
    match (a, b) {
        (UnsafeVal::Int(a), UnsafeVal::Int(b)) => Ok(a < b),
        (UnsafeVal::Float(a), UnsafeVal::Float(b)) => Ok(a < b),
        (UnsafeVal::Float(a), UnsafeVal::Int(b)) => Ok(a < (b as f64)),
        (UnsafeVal::Int(a), UnsafeVal::Float(b)) => Ok((a as f64) < b),
        (_, UnsafeVal::Int(_)) | (_, UnsafeVal::Float(_)) => Err(VmError::TypeError {
            context: "<",
            expected: "int or float",
            actual: a.type_name(),
            value: a.format_quoted(vm).to_string(),
        }),
        (_, _) => Err(VmError::TypeError {
            context: "<",
            expected: "int or float",
            actual: b.type_name(),
            value: b.format_quoted(vm).to_string(),
        }),
    }
}

pub fn less_impl(vm: &Vm, args: &[UnsafeVal]) -> VmResult<ValBuilder<'static>> {
    match args {
        [] | [_] => Ok(ValBuilder::new_bool(true)),
        [a, b] => Ok(ValBuilder::new_bool(less_two_impl(vm, *a, *b)?)),
        [a, b, ..] => match less_two_impl(vm, *a, *b)? {
            true => less_impl(vm, &args[1..]),
            false => Ok(ValBuilder::new_bool(false)),
        },
    }
}

pub fn less<'a>(ctx: NativeFunctionContext) -> VmResult<ValBuilder<'a>> {
    less_impl(ctx.vm(), ctx.args())
}

pub fn truthy_p<'a>(ctx: NativeFunctionContext) -> VmResult<ValBuilder<'a>> {
    match ctx.args() {
        [v] => match v {
            UnsafeVal::Void | UnsafeVal::Bool(false) => Ok(ValBuilder::new_bool(false)),
            _ => Ok(ValBuilder::new_bool(true)),
        },
        args => Err(VmError::ArityError {
            function: "truthy?".into(),
            expected: 1,
            actual: args.len(),
        }),
    }
}

pub fn not<'a>(ctx: NativeFunctionContext) -> VmResult<ValBuilder<'a>> {
    match ctx.args() {
        [UnsafeVal::Bool(b)] => Ok(ValBuilder::new_bool(!b)),
        [v] => Err(VmError::TypeError {
            context: "function not",
            expected: UnsafeVal::BOOL_TYPE_NAME,
            actual: v.type_name(),
            value: v.format_quoted(ctx.vm()).to_string(),
        }),
        args => Err(VmError::ArityError {
            function: "not".into(),
            expected: 1,
            actual: args.len(),
        }),
    }
}

pub fn string_join(mut ctx: NativeFunctionContext) -> VmResult<ValBuilder> {
    let args = ctx.args();
    let (strings, separator) = match args {
        [] => {
            return Err(VmError::ArityError {
                function: "string-join".into(),
                expected: 1,
                actual: 0,
            })
        }
        [UnsafeVal::List(list)] => (*list, ""),
        [v] => {
            return Err(VmError::TypeError {
                context: "string-join arg(idx=0)",
                expected: UnsafeVal::LIST_TYPE_NAME,
                actual: v.type_name(),
                value: v.format_quoted(ctx.vm()).to_string(),
            });
        }
        [UnsafeVal::List(list), UnsafeVal::String(string)] => {
            (*list, ctx.vm().objects.get_str(*string))
        }
        [_, v] => {
            return Err(VmError::TypeError {
                context: "string-join arg(idx=1)",
                expected: UnsafeVal::STRING_TYPE_NAME,
                actual: v.type_name(),
                value: v.format_quoted(ctx.vm()).to_string(),
            });
        }
        _ => {
            return Err(VmError::ArityError {
                function: "string-join".into(),
                expected: 2,
                actual: args.len(),
            })
        }
    };
    let mut result = CompactString::default();
    for (idx, string_id) in ctx.vm().objects.get_list(strings).iter().enumerate() {
        if idx > 0 {
            result.push_str(separator);
        }
        match string_id {
            UnsafeVal::String(string_id) => {
                result.push_str(ctx.vm().objects.get_str(*string_id));
            }
            _ => {
                return Err(VmError::TypeError {
                    context: "string-join arg(idx=0)",
                    expected: UnsafeVal::STRING_TYPE_NAME,
                    actual: string_id.type_name(),
                    value: string_id.format_quoted(ctx.vm()).to_string(),
                })
            }
        };
    }
    // Unsafe OK: Value is returned immediately.
    Ok(unsafe { ctx.new_string(result) })
}

pub fn new_box(mut ctx: NativeFunctionContext) -> VmResult<ValBuilder> {
    match ctx.args() {
        // Unsafe OK: Returns immediately so no garbage collection may happen.
        [v] => Ok(unsafe { ctx.new_mutable_box(*v) }),
        args => Err(VmError::ArityError {
            function: "new-box".into(),
            expected: 1,
            actual: args.len(),
        }),
    }
}

pub fn set_box(mut ctx: NativeFunctionContext) -> VmResult<ValBuilder> {
    match ctx.args() {
        // Unsafe OK: This is for sure safe...
        [UnsafeVal::MutableBox(id), v] => unsafe {
            let id = *id;
            let v = *v;
            let vm = ctx.vm_mut();
            Ok(ValBuilder::new_internal(vm.objects.set_mutable_box(id, v)))
        },
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

pub fn unbox(ctx: NativeFunctionContext) -> VmResult<ValBuilder> {
    match ctx.args() {
        // Unsafe OK: This is for sure safe...
        [UnsafeVal::MutableBox(id)] => unsafe {
            Ok(ValBuilder::new_internal(
                *ctx.vm().objects.get_mutable_box(*id),
            ))
        },
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

pub fn list(mut ctx: NativeFunctionContext) -> VmResult<ValBuilder> {
    let list = ctx.args().to_vec();
    // Unsafe OK: Value is returned immediately.
    Ok(unsafe { ctx.new_list(list) })
}

pub fn working_directory(mut ctx: NativeFunctionContext) -> VmResult<ValBuilder> {
    let arg_len = ctx.arg_len();
    if arg_len != 0 {
        return Err(VmError::ArityError {
            function: "working-directory".into(),
            expected: 0,
            actual: arg_len,
        });
    }
    let working_directory: CompactString = match std::env::current_dir() {
        Ok(path) => path.to_string_lossy().into(),
        // Untested OK: It is hard to create a working directory error and is not common.
        Err(err) => return Err(VmError::CustomError(err.to_string())),
    };
    // Unsafe OK: Value is returned immediately.
    Ok(unsafe { ctx.new_string(working_directory) })
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn equal_with_wrong_number_of_args_produces_arity_error() {
        let mut vm = Vm::default();
        assert_eq!(
            vm.eval_str("(=)").unwrap_err(),
            VmError::ArityError {
                function: "=".into(),
                expected: 2,
                actual: 0,
            },
        );
        assert_eq!(
            vm.eval_str("(= 1)").unwrap_err(),
            VmError::ArityError {
                function: "=".into(),
                expected: 2,
                actual: 1,
            },
        );
        assert_eq!(
            vm.eval_str("(= 1 2 3)").unwrap_err(),
            VmError::ArityError {
                function: "=".into(),
                expected: 2,
                actual: 3,
            },
        );
    }

    #[test]
    fn equal_with_equal_items_returns_true() {
        let mut vm = Vm::default();
        assert_eq!(
            vm.eval_str("(= false false)").unwrap().as_bool(),
            Some(true)
        );
        assert_eq!(vm.eval_str("(= 1 1)").unwrap().as_bool(), Some(true));
        assert_eq!(vm.eval_str("(= 2.0 2.0)").unwrap().as_bool(), Some(true));
        assert_eq!(
            vm.eval_str("(= \"string\" \"string\")").unwrap().as_bool(),
            Some(true)
        );
        assert_eq!(
            vm.eval_str("(= (list \"list\") (list \"list\"))")
                .unwrap()
                .as_bool(),
            Some(true)
        );
        vm.eval_str("(define (foo) 42)").unwrap();
        assert_eq!(vm.eval_str("(= foo foo)").unwrap().as_bool(), Some(true));
        assert_eq!(
            vm.eval_str("(= (foo) (foo))").unwrap().as_bool(),
            Some(true)
        );
        assert_eq!(vm.eval_str("(= + +)").unwrap().as_bool(), Some(true));

        vm.values.insert("void1".into(), ().into());
        vm.values.insert("void2".into(), ().into());
        assert_eq!(
            vm.eval_str("(= void1 void2)").unwrap().as_bool(),
            Some(true)
        );
    }

    #[test]
    fn equal_with_different_items_returns_true() {
        let mut vm = Vm::default();
        assert_eq!(vm.eval_str("(= 1 1.0)").unwrap().as_bool(), Some(false),);
        assert_eq!(
            vm.eval_str("(= true false)").unwrap().as_bool(),
            Some(false)
        );
        assert_eq!(vm.eval_str("(= 1 2)").unwrap().as_bool(), Some(false));
        assert_eq!(vm.eval_str("(= 1.0 2.0)").unwrap().as_bool(), Some(false));
        assert_eq!(
            vm.eval_str("(= \"string\" \"other\")").unwrap().as_bool(),
            Some(false)
        );
        assert_eq!(
            vm.eval_str("(= (list) (list 0))").unwrap().as_bool(),
            Some(false)
        );
        assert_eq!(
            vm.eval_str("(= (list \"list\" 1) (list \"list\" 2))")
                .unwrap()
                .as_bool(),
            Some(false)
        );
        vm.eval_str("(define (foo) 42) (define (bar) 42)").unwrap();
        assert_eq!(vm.eval_str("(= foo bar)").unwrap().as_bool(), Some(false));
        assert_eq!(vm.eval_str("(= + <)").unwrap().as_bool(), Some(false));
    }

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
                expected: UnsafeVal::INT_TYPE_NAME,
                actual: UnsafeVal::STRING_TYPE_NAME,
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
                actual: UnsafeVal::STRING_TYPE_NAME,
                value: "\"blue\"".to_string(),
            }
        );
        let got = vm.eval_str("(< \"blue\" 2.0)").unwrap_err();
        assert_eq!(
            got,
            VmError::TypeError {
                context: "<",
                expected: "int or float",
                actual: UnsafeVal::STRING_TYPE_NAME,
                value: "\"blue\"".to_string(),
            }
        );
        let got = vm.eval_str("(< -1 \"fish\" 2)").unwrap_err();
        assert_eq!(
            got,
            VmError::TypeError {
                context: "<",
                expected: "int or float",
                actual: UnsafeVal::STRING_TYPE_NAME,
                value: "\"fish\"".to_string(),
            }
        );
    }

    #[test]
    fn truthy_p_is_false_on_false_and_void() {
        let mut vm = Vm::default();
        assert_eq!(
            vm.eval_str("(truthy? false)").unwrap().as_bool(),
            Some(false)
        );
        assert_eq!(
            vm.eval_str("(truthy? void)").unwrap().as_bool(),
            Some(false)
        );
    }

    #[test]
    fn truthy_p_is_true_on_all_non_false_void_values() {
        let mut vm = Vm::default();
        assert_eq!(vm.eval_str("(truthy? true)").unwrap().as_bool(), Some(true));
        assert_eq!(vm.eval_str("(truthy? 1)").unwrap().as_bool(), Some(true));
        assert_eq!(vm.eval_str("(truthy? 1.0)").unwrap().as_bool(), Some(true));
        assert_eq!(vm.eval_str("(truthy? \"\")").unwrap().as_bool(), Some(true));
        assert_eq!(
            vm.eval_str("(truthy? truthy?)").unwrap().as_bool(),
            Some(true)
        );
    }

    #[test]
    fn truthy_p_with_wrong_number_of_args_produces_error() {
        let mut vm = Vm::default();
        assert_eq!(
            vm.eval_str("(truthy?)").unwrap_err(),
            VmError::ArityError {
                function: "truthy?".into(),
                expected: 1,
                actual: 0,
            }
        );
        assert_eq!(
            vm.eval_str("(truthy? 0 1)").unwrap_err(),
            VmError::ArityError {
                function: "truthy?".into(),
                expected: 1,
                actual: 2,
            }
        );
    }

    #[test]
    fn not_inverts_bool() {
        let mut vm = Vm::default();
        assert_eq!(vm.eval_str("(not true)").unwrap().as_bool(), Some(false));
        assert_eq!(vm.eval_str("(not false)").unwrap().as_bool(), Some(true));
    }

    #[test]
    fn not_with_wrong_args_produces_error() {
        let mut vm = Vm::default();
        assert_eq!(
            vm.eval_str("(not)").unwrap_err(),
            VmError::ArityError {
                function: "not".into(),
                expected: 1,
                actual: 0,
            }
        );
        assert_eq!(
            vm.eval_str("(not true false)").unwrap_err(),
            VmError::ArityError {
                function: "not".into(),
                expected: 1,
                actual: 2,
            }
        );
        assert_eq!(
            vm.eval_str("(not void)").unwrap_err(),
            VmError::TypeError {
                context: "function not",
                expected: UnsafeVal::BOOL_TYPE_NAME,
                actual: UnsafeVal::VOID_TYPE_NAME,
                value: "<void>".into(),
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
                function: "string-join".into(),
                expected: 1,
                actual: 0,
            },
        );
        assert_eq!(
            vm.eval_str("(string-join 1 2 3)").unwrap_err(),
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
        assert_eq!(
            vm.eval_str("(string-join 2)").unwrap_err(),
            VmError::TypeError {
                context: "string-join arg(idx=0)",
                expected: UnsafeVal::LIST_TYPE_NAME,
                actual: UnsafeVal::INT_TYPE_NAME,
                value: "2".to_string(),
            },
        );
        assert_eq!(
            vm.eval_str("(string-join (list \"ok string\" 42))")
                .unwrap_err(),
            VmError::TypeError {
                context: "string-join arg(idx=0)",
                expected: UnsafeVal::STRING_TYPE_NAME,
                actual: UnsafeVal::INT_TYPE_NAME,
                value: "42".to_string(),
            },
        );
        assert_eq!(
            vm.eval_str("(string-join (list) 3)").unwrap_err(),
            VmError::TypeError {
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
        assert_eq!(vm.eval_str("val").unwrap().as_str(), None);
    }

    #[test]
    fn get_box_returns_value_inside_box() {
        let mut vm = Vm::default();
        vm.eval_str("(define val (new-box \"foo\"))").unwrap();
        assert_eq!(vm.eval_str("(unbox val)").unwrap().as_str(), Some("foo"));
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
        assert_eq!(vm.eval_str("val").unwrap().as_str(), None);
        assert_eq!(vm.eval_str("(unbox val)").unwrap().as_str(), Some("foo"));

        vm.eval_str("(set-box! val \"bar\")").unwrap();
        assert_eq!(vm.eval_str("(unbox val)").unwrap().as_str(), Some("bar"));
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

    #[test]
    fn working_directory_with_args_produces_arity_error() {
        let mut vm = Vm::default();
        assert_eq!(
            vm.eval_str("(working-directory 1)").unwrap_err(),
            VmError::ArityError {
                function: "working-directory".into(),
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
