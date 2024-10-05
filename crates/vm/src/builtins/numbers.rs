use crate::{
    error::{VmError, VmResult},
    val::{NativeFunctionContext, UnsafeVal, Val, ValBuilder},
    Vm,
};

enum Number {
    Int(i64),
    Float(f64),
}

impl From<Number> for Val<'static> {
    fn from(v: Number) -> Val<'static> {
        match v {
            Number::Int(x) => Val::new_int(x),
            Number::Float(x) => Val::new_float(x),
        }
    }
}

fn add_impl<'a>(
    vm: &Vm,
    context: &'static str,
    args: impl 'a + Iterator<Item = &'a Val<'a>>,
) -> VmResult<Number> {
    let mut int_sum: i64 = 0;
    let mut float_sum: f64 = 0.0;
    for arg in args {
        // Unsafe OK: Using field values right away without any garbage collection.
        // TODO: Consider getting the number through [Val] directly.
        match unsafe { arg.as_unsafe_val() } {
            UnsafeVal::Int(x) => int_sum += x,
            UnsafeVal::Float(x) => {
                float_sum += x;
            }
            _ => {
                return Err(VmError::TypeError {
                    src: None,
                    context,
                    expected: "int or float",
                    actual: arg.type_name(),
                    value: arg.format_quoted(vm).to_string(),
                })
            }
        }
    }
    if float_sum != 0.0 {
        Ok(Number::Float(float_sum + int_sum as f64))
    } else {
        Ok(Number::Int(int_sum))
    }
}

pub fn add<'a>(ctx: NativeFunctionContext<'a>, args: &[Val<'a>]) -> VmResult<ValBuilder<'a>> {
    let res = add_impl(ctx.vm(), "+", args.iter())?;
    Ok(ValBuilder::new(res.into()))
}

fn negate(vm: &Vm, context: &'static str, v: Val) -> VmResult<Number> {
    match unsafe { v.as_unsafe_val() } {
        UnsafeVal::Int(x) => Ok(Number::Int(-x)),
        UnsafeVal::Float(x) => Ok(Number::Float(-x)),
        _ => Err(VmError::TypeError {
            src: None,
            context,
            expected: "int or float",
            actual: v.type_name(),
            value: v.format_quoted(vm).to_string(),
        }),
    }
}

pub fn subtract<'a>(ctx: NativeFunctionContext, args: &[Val]) -> VmResult<ValBuilder<'a>> {
    let vm = ctx.vm();
    match args {
        [v] => negate(vm, "-", *v).map(|x| ValBuilder::new(x.into())),
        [v, rest @ ..] => {
            let rest_sum = add_impl(vm, "-", rest.iter())?;
            let negated_rest = negate(vm, "-", rest_sum.into())?;
            let ans = add_impl(vm, "-", [*v, negated_rest.into()].iter())?;
            Ok(ValBuilder::new(ans.into()))
        }
        [] => Err(VmError::ArityError {
            function: "-".into(),
            expected: 1,
            actual: 0,
        }),
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
            src: None,
            context: "<",
            expected: "int or float",
            actual: a.type_name(),
            value: a.format_quoted(vm).to_string(),
        }),
        (_, _) => Err(VmError::TypeError {
            src: None,
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

pub fn less<'a>(ctx: NativeFunctionContext, args: &[Val]) -> VmResult<ValBuilder<'a>> {
    let res = less_impl(ctx.vm(), args)?;
    Ok(Val::new_bool(res).into())
}

#[cfg(test)]
mod tests {
    use crate::parser::span::Span;

    use super::*;

    #[test]
    fn add_with_no_args_is_int_0() {
        let mut vm = Vm::default();
        let got = vm.eval_str("(+)").unwrap();
        assert_eq!(got.try_int().unwrap(), 0);
        assert!(got.try_float().is_err());
    }

    #[test]
    fn add_with_nonnumber_is_type_error() {
        let mut vm = Vm::default();
        let src = "(+ 1 2 \"fish\")";
        let got = vm.eval_str(src).unwrap_err();
        assert_eq!(
            got,
            VmError::TypeError {
                src: Some(Span::new(0, 14).with_src(src.into())),
                context: "+",
                expected: "int or float",
                actual: UnsafeVal::STRING_TYPE_NAME,
                value: "\"fish\"".to_string(),
            }
        );
    }

    #[test]
    fn add_ints_returns_int() {
        let mut vm = Vm::default();
        let got = vm.eval_str("(+ 1 2 3)").unwrap();
        assert_eq!(got.try_int().unwrap(), 6);
        assert!(got.try_float().is_err());
    }

    #[test]
    fn add_floats_returns_floats() {
        let mut vm = Vm::default();
        let got = vm.eval_str("(+ 1.0 2.0 3.0)").unwrap();
        assert_eq!(got.try_float().unwrap(), 6.0);
        assert!(got.try_int().is_err());
    }

    #[test]
    fn add_ints_and_floats_returns_floats() {
        let mut vm = Vm::default();
        let got = vm.eval_str("(+ 1 2.0 3)").unwrap();
        assert_eq!(got.try_float().unwrap(), 6.0);
        assert!(got.try_int().is_err());
    }

    #[test]
    fn subtract_with_no_args_returns_error() {
        let mut vm = Vm::default();
        assert_eq!(
            vm.eval_str("(-)").unwrap_err(),
            VmError::ArityError {
                function: "-".into(),
                expected: 1,
                actual: 0,
            }
        );
    }

    #[test]
    fn subtract_with_wrong_args_returns_error() {
        let mut vm = Vm::default();
        let src = "(- \"string\")";
        assert_eq!(
            vm.eval_str(src).unwrap_err(),
            VmError::TypeError {
                src: Some(Span::new(0, 12).with_src(src.into())),
                context: "-",
                expected: "int or float",
                actual: UnsafeVal::STRING_TYPE_NAME,
                value: "\"string\"".into(),
            }
        );
        let src = "(- 0 (list))";
        assert_eq!(
            vm.eval_str(src).unwrap_err(),
            VmError::TypeError {
                src: Some(Span::new(0, 12).with_src(src.into())),
                context: "-",
                expected: "int or float",
                actual: UnsafeVal::LIST_TYPE_NAME,
                value: "()".into(),
            }
        );
    }

    #[test]
    fn subtract_with_single_number_negates() {
        let mut vm = Vm::default();
        assert_eq!(vm.eval_str("(- 1)").unwrap().try_int().unwrap(), -1);
        assert_eq!(vm.eval_str("(- 1.0)").unwrap().try_float().unwrap(), -1.0);
    }

    #[test]
    fn subtract_with_multiple_number_subtracts_from_first_arg() {
        let mut vm = Vm::default();
        assert_eq!(vm.eval_str("(- 1 2 3)").unwrap().try_int().unwrap(), -4);
        assert_eq!(
            vm.eval_str("(- 1 2.0 3)").unwrap().try_float().unwrap(),
            -4.0
        );
        assert_eq!(
            vm.eval_str("(- 1.0 2 3)").unwrap().try_float().unwrap(),
            -4.0
        );
    }

    #[test]
    fn less_with_no_args_is_true() {
        let mut vm = Vm::default();
        let got = vm.eval_str("(<)").unwrap();
        assert!(got.try_bool().unwrap());
    }

    #[test]
    fn less_with_single_arg_is_true() {
        let mut vm = Vm::default();
        let got = vm.eval_str("(< 1)").unwrap();
        assert!(got.try_bool().unwrap());
    }

    #[test]
    fn less_with_increasing_ordered_args_is_true() {
        let mut vm = Vm::default();
        let got = vm.eval_str("(< -1 0 1 1.2 1.8 2)").unwrap();
        assert!(got.try_bool().unwrap());
    }

    #[test]
    fn less_with_unordered_args_is_false() {
        let mut vm = Vm::default();
        let got = vm.eval_str("(< -1 0 -0.1 1.2 2)").unwrap();
        assert!(!got.try_bool().unwrap());
    }

    #[test]
    fn less_with_nonumber_args_is_type_error() {
        let mut vm = Vm::default();
        let src = "(< \"blue\" 2)";
        assert_eq!(
            vm.eval_str(src).unwrap_err(),
            VmError::TypeError {
                src: Some(Span::new(0, 12).with_src(src.into())),
                context: "<",
                expected: "int or float",
                actual: UnsafeVal::STRING_TYPE_NAME,
                value: "\"blue\"".to_string(),
            }
        );
        let src = "(< \"blue\" 2.0)";
        assert_eq!(
            vm.eval_str(src).unwrap_err(),
            VmError::TypeError {
                src: Some(Span::new(0, 14).with_src(src.into())),
                context: "<",
                expected: "int or float",
                actual: UnsafeVal::STRING_TYPE_NAME,
                value: "\"blue\"".to_string(),
            }
        );
        let src = "(< -1 \"fish\" 2)";
        assert_eq!(
            vm.eval_str(src).unwrap_err(),
            VmError::TypeError {
                src: Some(Span::new(0, 15).with_src(src.into())),
                context: "<",
                expected: "int or float",
                actual: UnsafeVal::STRING_TYPE_NAME,
                value: "\"fish\"".to_string(),
            }
        );
    }
}
