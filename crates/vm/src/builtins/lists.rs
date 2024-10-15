use crate::{
    error::{VmError, VmResult},
    val::{NativeFunctionContext, UnsafeVal, ValBuilder},
};

pub fn list(ctx: NativeFunctionContext<'_>) -> VmResult<ValBuilder<'_>> {
    let args: Vec<_> = ctx.args().map(|x| x.as_unsafe_val()).collect();
    Ok(unsafe { ctx.new_list(args) })
}

pub fn list_length(ctx: NativeFunctionContext<'_>) -> VmResult<ValBuilder<'_>> {
    match ctx.arg_count() {
        1 => match ctx.arg(0).unwrap().try_list(ctx.vm()) {
            Ok(list) => Ok(ValBuilder::new((list.len() as i64).into())),
            Err(v) => Err(VmError::TypeError {
                src: None,
                context: "list-length",
                expected: UnsafeVal::LIST_TYPE_NAME,
                actual: v.type_name(),
                value: v.format_quoted(ctx.vm()).to_string(),
            }),
        },
        n => Err(VmError::ArityError {
            function: "list-length".into(),
            expected: 1,
            actual: n,
        }),
    }
}

#[cfg(test)]
mod tests {
    use crate::{parser::span::Span, Vm};

    use super::*;

    #[test]
    fn list_length_with_wrong_args_returns_error() {
        let mut vm = Vm::default();
        assert_eq!(
            vm.eval_str("(list-length)").unwrap_err(),
            VmError::ArityError {
                function: "list-length".into(),
                expected: 1,
                actual: 0
            }
        );
        assert_eq!(
            vm.eval_str("(list-length (list) 0)").unwrap_err(),
            VmError::ArityError {
                function: "list-length".into(),
                expected: 1,
                actual: 2
            }
        );
        let src = "(list-length 0)";
        assert_eq!(
            vm.eval_str(src).unwrap_err(),
            VmError::TypeError {
                src: Some(Span::new(0, 15).with_src(src.into())),
                context: "list-length",
                expected: "list",
                actual: "int",
                value: "0".into(),
            }
        );
    }

    #[test]
    fn list_length_on_empty_list_returns_zero() {
        let mut vm = Vm::default();
        assert_eq!(
            vm.eval_str("(list-length (list))")
                .unwrap()
                .try_int()
                .unwrap(),
            0
        );
    }

    #[test]
    fn list_length_on_list_returns_its_length() {
        let mut vm = Vm::default();
        assert_eq!(
            vm.eval_str("(list-length (list 1 2 3 4 5))")
                .unwrap()
                .try_int()
                .unwrap(),
            5
        );
    }
}
