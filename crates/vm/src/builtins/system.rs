use std::process::Command;

use compact_str::CompactString;
use log::*;

use crate::{
    error::{VmError, VmResult},
    val::{NativeFunctionContext, UnsafeVal, Val, ValBuilder},
};

pub fn working_directory<'a>(
    ctx: NativeFunctionContext<'a>,
    args: &[Val<'a>],
) -> VmResult<ValBuilder<'a>> {
    match args {
        [] => {
            let working_directory: CompactString = match std::env::current_dir() {
                Ok(path) => path.to_string_lossy().into(),
                // Untested OK: It is hard to create a working directory error and is not common.
                Err(err) => return Err(VmError::CustomError(err.to_string())),
            };
            Ok(ctx.new_string(working_directory))
        }
        _ => Err(VmError::ArityError {
            function: "working-directory".into(),
            expected: 0,
            actual: args.len(),
        }),
    }
}

pub fn command<'a>(ctx: NativeFunctionContext<'a>, args: &[Val<'a>]) -> VmResult<ValBuilder<'a>> {
    match args {
        [cmd, rest @ ..] => {
            let cmd_str = cmd.try_str(ctx.vm()).map_err(|v| VmError::TypeError {
                src: None,
                context: "command arg(idx=0)",
                expected: UnsafeVal::STRING_TYPE_NAME,
                actual: v.type_name(),
                value: v.format_quoted(ctx.vm()).to_string(),
            })?;
            let mut cmd = Command::new(cmd_str);
            for arg in rest {
                let arg_str = arg.try_str(ctx.vm()).map_err(|v| VmError::TypeError {
                    src: None,
                    context: "command arg(idx>0)",
                    expected: UnsafeVal::STRING_TYPE_NAME,
                    actual: v.type_name(),
                    value: v.format_quoted(ctx.vm()).to_string(),
                })?;
                cmd.arg(arg_str);
            }
            let output = cmd.output().map_err(|err| {
                VmError::CustomError(format!("failed to run command {cmd_str}: {err}"))
            })?;
            if !output.stderr.is_empty() {
                let err = String::from_utf8_lossy(&output.stderr);
                error!("Command {cmd_str}: {err}");
            }
            if !output.status.success() {
                return Err(VmError::CustomError(format!(
                    "command {cmd_str} exited with code {code:?}",
                    code = output.status.code()
                )));
            }
            let output = ctx.new_string(CompactString::from_utf8_lossy(&output.stdout));
            Ok(output)
        }
        [] => Err(VmError::ArityError {
            function: "command".into(),
            expected: 1,
            actual: 0,
        }),
    }
}

#[cfg(test)]
mod tests {
    use crate::{parser::span::Span, Vm};

    use super::*;

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
        assert_eq!(got.try_str().unwrap(), working_directory.as_str());
    }

    #[test]
    fn command_can_execute() {
        let mut vm = Vm::default();
        let src = "(command \"ls\" \"benches\")";
        assert_eq!(vm.eval_str(src).unwrap().try_str().unwrap(), "bench.rs\n");
    }

    #[test]
    fn command_with_no_args_returns_error() {
        let mut vm = Vm::default();
        let src = "(command)";
        assert_eq!(
            vm.eval_str(src).unwrap_err(),
            VmError::ArityError {
                function: "command".into(),
                expected: 1,
                actual: 0
            }
        );
    }

    #[test]
    fn command_with_non_string_arg_returns_error() {
        let mut vm = Vm::default();

        let src = "(command 1)";
        assert_eq!(
            vm.eval_str(src).unwrap_err(),
            VmError::TypeError {
                src: Some(Span::new(0, 11).with_src(src.into())),
                context: "command arg(idx=0)",
                expected: UnsafeVal::STRING_TYPE_NAME,
                actual: UnsafeVal::INT_TYPE_NAME,
                value: "1".into(),
            }
        );

        let src = "(command \"ls\" 1)";
        assert_eq!(
            vm.eval_str(src).unwrap_err(),
            VmError::TypeError {
                src: Some(Span::new(0, 16).with_src(src.into())),
                context: "command arg(idx>0)",
                expected: UnsafeVal::STRING_TYPE_NAME,
                actual: UnsafeVal::INT_TYPE_NAME,
                value: "1".into(),
            }
        );
    }

    #[test]
    fn command_that_does_not_exist_returns_error() {
        let mut vm = Vm::default();
        let src = "(command \"does-not-exist-1234\")";
        assert_eq!(
            vm.eval_str(src).unwrap_err(),
            VmError::CustomError(
                "failed to run command does-not-exist-1234: No such file or directory (os error 2)"
                    .into()
            ),
        );
    }
}
