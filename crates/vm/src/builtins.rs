use compact_str::CompactString;

use crate::{
    error::{VmError, VmResult},
    val::{NativeFunction, NativeFunctionContext, UnsafeVal, Val, ValBuilder},
    Vm,
};

pub const BUILTINS: &[(&str, NativeFunction)] = &[
    ("=", equal),
    ("+", add),
    ("<", less),
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
        [a, b] => {
            // Unsafe OK: [equal_imp] holds the a reference to the VM so it can't run garbage
            // collection.
            let (a, b) = unsafe { (a.as_unsafe_val(), b.as_unsafe_val()) };
            Ok(Val::new_bool(equal_impl(ctx.vm(), a, b)).into())
        }
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
        // Unsafe OK: Using field values right away without any garbage collection.
        // TODO: Consider getting the number through [Val] directly.
        match unsafe { arg.as_unsafe_val() } {
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
        Ok(Val::new_float(float_sum + int_sum as f64).into())
    } else {
        Ok(Val::new_int(int_sum).into())
    }
}

fn less_two_impl(vm: &Vm, a: Val, b: Val) -> VmResult<bool> {
    // Unsafe OK: Only gets basic types like int and float.
    let (a, b) = unsafe { (a.as_unsafe_val(), b.as_unsafe_val()) };
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

pub fn less_impl(vm: &Vm, args: &[Val]) -> VmResult<bool> {
    match args {
        [] | [_] => Ok(true),
        [a, b] => Ok(less_two_impl(vm, *a, *b)?),
        [a, b, ..] => match less_two_impl(vm, *a, *b)? {
            true => less_impl(vm, &args[1..]),
            false => Ok(false),
        },
    }
}

pub fn less<'a>(ctx: NativeFunctionContext) -> VmResult<ValBuilder<'a>> {
    let res = less_impl(ctx.vm(), ctx.args())?;
    Ok(Val::new_bool(res).into())
}

pub fn not<'a>(ctx: NativeFunctionContext) -> VmResult<ValBuilder<'a>> {
    match ctx.args() {
        [v] => Ok(Val::new_bool(!v.is_truthy()).into()),
        args => Err(VmError::ArityError {
            function: "not".into(),
            expected: 1,
            actual: args.len(),
        }),
    }
}

pub fn string_join(ctx: NativeFunctionContext) -> VmResult<ValBuilder> {
    if ctx.args_len() > 2 {
        return Err(VmError::ArityError {
            function: "string-join".into(),
            expected: 2,
            actual: ctx.args_len(),
        });
    }
    let mut args = ctx.args().into_iter();
    let maybe_list = args
        .next()
        .ok_or_else(|| VmError::ArityError {
            function: "string-join".into(),
            expected: 1,
            actual: 0,
        })?
        .try_list(ctx.vm())
        .map_err(|v| VmError::TypeError {
            context: "string-join arg(idx=0)",
            expected: UnsafeVal::LIST_TYPE_NAME,
            actual: v.type_name(),
            value: v.format_quoted(ctx.vm()).to_string(),
        })?;
    let separator = match args.next() {
        None => "",
        Some(v) => v.as_str(ctx.vm()).ok_or_else(|| VmError::TypeError {
            context: "string-join arg(idx=1)",
            expected: UnsafeVal::STRING_TYPE_NAME,
            actual: v.type_name(),
            value: v.format_quoted(ctx.vm()).to_string(),
        })?,
    };
    let mut result = CompactString::default();
    for (idx, maybe_string_val) in maybe_list.iter().enumerate() {
        if idx > 0 {
            result.push_str(separator);
        }
        let s = maybe_string_val
            .try_str(ctx.vm())
            .map_err(|v| VmError::TypeError {
                context: "string-join arg(idx=0) list subelement",
                expected: UnsafeVal::STRING_TYPE_NAME,
                actual: v.type_name(),
                value: v.format_quoted(ctx.vm()).to_string(),
            })?;
        result.push_str(s);
    }
    Ok(ctx.new_string(result))
}

pub fn new_box(ctx: NativeFunctionContext) -> VmResult<ValBuilder> {
    match ctx.args() {
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

pub fn set_box(mut ctx: NativeFunctionContext) -> VmResult<ValBuilder> {
    // TODO: Use safe API.
    match unsafe { ctx.raw_args() } {
        // Unsafe OK: This is for sure safe...
        [UnsafeVal::MutableBox(id), inner_val] => {
            let (id, inner_val) = (*id, *inner_val);
            let boxed_val = ctx.vm_mut().objects.set_mutable_box(id, inner_val);
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

pub fn unbox(ctx: NativeFunctionContext) -> VmResult<ValBuilder> {
    // TODO: Use safe API.
    match unsafe { ctx.raw_args() } {
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

pub fn list(ctx: NativeFunctionContext) -> VmResult<ValBuilder> {
    // Unsafe OK: `ctx.args()` guarantees values will not be garbage collected.
    let list = unsafe { ctx.raw_args().to_vec() };
    Ok(unsafe { ctx.new_list(list) })
}

pub fn working_directory(ctx: NativeFunctionContext) -> VmResult<ValBuilder> {
    let arg_len = ctx.args_len();
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
    Ok(ctx.new_string(working_directory))
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
    fn not_inverts_bool() {
        let mut vm = Vm::default();
        assert_eq!(vm.eval_str("(not true)").unwrap().as_bool(), Some(false));
        assert_eq!(vm.eval_str("(not false)").unwrap().as_bool(), Some(true));
    }

    #[test]
    fn not_with_wrong_not_just_one_arg_produces_arity_error() {
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
    }

    #[test]
    fn not_with_void_values_returns_true() {
        let mut vm = Vm::default();
        assert_eq!(vm.eval_str("(not void)").unwrap().as_bool(), Some(true));
    }

    #[test]
    fn not_with_truthy_values_returns_true() {
        let mut vm = Vm::default();
        assert_eq!(vm.eval_str("(not true)").unwrap().as_bool(), Some(false));
        assert_eq!(vm.eval_str("(not 1)").unwrap().as_bool(), Some(false));
        assert_eq!(vm.eval_str("(not 1.0)").unwrap().as_bool(), Some(false));
        assert_eq!(vm.eval_str("(not \"\")").unwrap().as_bool(), Some(false));
        assert_eq!(vm.eval_str("(not not)").unwrap().as_bool(), Some(false));
        assert_eq!(vm.eval_str("(not (list))").unwrap().as_bool(), Some(false));
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
                context: "string-join arg(idx=0) list subelement",
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
