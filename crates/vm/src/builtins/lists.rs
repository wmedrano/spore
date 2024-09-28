use crate::{
    error::{VmError, VmResult},
    val::{NativeFunctionContext, UnsafeVal, Val, ValBuilder},
};

pub fn list<'a>(ctx: NativeFunctionContext<'a>, args: &[Val]) -> VmResult<ValBuilder<'a>> {
    Ok(unsafe { ctx.new_list(args) })
}

pub fn list_length<'a>(ctx: NativeFunctionContext<'a>, args: &[Val]) -> VmResult<ValBuilder<'a>> {
    match args {
        [arg] => match arg.try_list(ctx.vm()) {
            Ok(list) => Ok(ValBuilder::new((list.len() as i64).into())),
            Err(v) => Err(VmError::TypeError {
                context: "list-length",
                expected: UnsafeVal::LIST_TYPE_NAME,
                actual: v.type_name(),
                value: v.format_quoted(ctx.vm()).to_string(),
            }),
        },
        _ => Err(VmError::ArityError {
            function: "list-length".into(),
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
    fn list_length_with_wrong_args_produces_error() {
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
        assert_eq!(
            vm.eval_str("(list-length 0)").unwrap_err(),
            VmError::TypeError {
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
