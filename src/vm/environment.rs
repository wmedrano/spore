use std::{collections::HashMap, rc::Rc};

use anyhow::{bail, Result};

use crate::parser::ast::Ast;

use super::{
    compiler::Compiler,
    types::{
        instruction::Instruction,
        proc::{ByteCodeIter, ByteCodeProc},
        symbol::Symbol,
        Val,
    },
};

/// An environment to evaluate bytecode on.
pub struct Environment {
    /// The registry of global values.
    pub globals: HashMap<Symbol, Val>,
    /// The processing stack.
    pub stack: Vec<Val>,
    /// Contains the current call frame. This includes the instructions that should be run and the
    /// base of the frame's stack.
    pub frames: Vec<Frame>,
}

pub struct Frame {
    /// The bytecode that is under execution in the frame.
    pub bytecode: ByteCodeIter,
    /// The index of the local stack.
    pub stack_start_idx: usize,
}

impl Environment {
    /// Evaluate an S-Expression string and return the last value. If there are no expression, than
    /// `Val::Void` is returned.
    pub fn eval_str(&mut self, s: &str) -> Result<Vec<Val>> {
        Ast::from_sexp_str(s)?
            .iter()
            .map(|ast| {
                let proc = Compiler::new(self).compile_and_finalize(ast)?;
                self.eval_bytecode(proc.into(), &[])
            })
            .collect()
    }

    /// Evaluate a sequence of bytecode.
    pub fn eval_bytecode(&mut self, proc: Rc<ByteCodeProc>, args: &[Val]) -> Result<Val> {
        self.prepare(proc, args);
        loop {
            let instruction = self
                .frames
                .last_mut()
                .and_then(|f| f.bytecode.next())
                .unwrap_or(Instruction::Return);
            match instruction {
                Instruction::PushVal(v) => self.execute_push_val(v.clone()),
                Instruction::Eval(n) => self.execute_eval_n(n)?,
                Instruction::JumpIf(n) => self.execute_jump_if(n)?,
                Instruction::Jump(n) => self.execute_jump(n),
                Instruction::GetVal(s) => self.execute_get_val(&s)?,
                Instruction::SetVal(s) => self.execute_set_val(s)?,
                Instruction::GetArg(n) => self.execute_get_arg(n),
                Instruction::Return => {
                    self.pop_frame()?;
                    if self.frames.is_empty() {
                        return Ok(self.stack.pop().unwrap_or_default());
                    }
                }
            }
        }
    }

    fn prepare(&mut self, proc: Rc<ByteCodeProc>, args: &[Val]) {
        self.frames.clear();
        self.stack.clear();
        self.stack.extend_from_slice(args);
        self.frames.push(Frame {
            bytecode: ByteCodeIter::from_proc(proc),
            stack_start_idx: 0,
        });
    }

    /// Pop the current frame. This truncates the local stack and replaces the top value of the
    /// stack with the return value. The return value is defined as the value at the top of the
    /// local stack.
    fn pop_frame(&mut self) -> Result<()> {
        let frame = self.frames.pop().unwrap();
        let return_val = if self.stack.len() > frame.stack_start_idx {
            self.stack.pop().unwrap_or_default()
        } else {
            Val::Void
        };
        self.stack.truncate(frame.stack_start_idx);
        self.stack.pop();
        self.stack.push(return_val);
        Ok(())
    }

    /// Get a value from the current environment.
    fn get_value(&self, sym: &Symbol) -> Option<Val> {
        self.globals.get(sym).cloned()
    }

    fn execute_get_arg(&mut self, n: usize) {
        let start_idx = self.frames.last().unwrap().stack_start_idx;
        let idx = start_idx + n;
        let v = self.stack.get(idx).unwrap().clone();
        self.stack.push(v);
    }

    fn execute_jump_if(&mut self, n: usize) -> Result<()> {
        let v = self.stack.pop().unwrap_or_default();
        if v.is_truthy()? {
            self.execute_jump(n);
        }
        Ok(())
    }

    fn execute_jump(&mut self, n: usize) {
        let frame = self.frames.last_mut().unwrap();
        frame.bytecode.jump(n);
    }

    fn execute_push_val(&mut self, v: Val) {
        self.stack.push(v);
    }

    fn execute_eval_n(&mut self, n: usize) -> Result<()> {
        let proc_idx = self.stack.len() - n;
        match &self.stack[proc_idx] {
            Val::ByteCodeProc(proc) => {
                let expected_args = proc.arg_count;
                let actual_args = n - 1;
                if expected_args != actual_args {
                    bail!(
                        "{name} expected {expected_args} but found {actual_args}",
                        name = proc.name
                    );
                }
                self.frames.push(Frame {
                    bytecode: ByteCodeIter::from_proc(proc.clone()),
                    stack_start_idx: proc_idx + 1,
                });
            }
            Val::NativeProc(proc) => {
                let stack_base = proc_idx + 1;
                let res = (proc.f)(&self.stack[stack_base..])?;
                self.stack.truncate(stack_base);
                *self.stack.last_mut().unwrap() = res;
            }
            v => bail!("expected procedure but found {v}"),
        };
        Ok(())
    }

    fn execute_get_val(&mut self, s: &Symbol) -> Result<()> {
        match self.get_value(s) {
            Some(v) => {
                self.stack.push(v);
            }
            None => bail!("{s} is not defined"),
        }
        Ok(())
    }

    fn execute_set_val(&mut self, s: Symbol) -> Result<()> {
        let v = self.stack.pop().unwrap();
        self.globals.insert(s, v);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::vm::{types::Number, Vm};

    use super::*;

    #[test]
    fn can_execute_ast() {
        assert_eq!(
            Vm::with_builtins()
                .build_env()
                .eval_str("(+ 1 2 (- 3 4))")
                .unwrap(),
            vec![Val::Number(Number::Int(2))]
        );
    }

    #[test]
    fn if_with_true_returns_first_expr_result() {
        assert_eq!(
            Vm::with_builtins()
                .build_env()
                .eval_str("(if true (* 10 2) (+ 10 2))")
                .unwrap(),
            vec![Val::Number(Number::Int(20))],
        );
    }

    #[test]
    fn if_with_false_returns_second_expr_result() {
        assert_eq!(
            Vm::with_builtins()
                .build_env()
                .eval_str("(if false (* 10 2) (+ 10 2))")
                .unwrap(),
            vec![Val::Number(Number::Int(12))],
        )
    }

    #[test]
    fn if_with_true_and_single_arm_returns_true() {
        assert_eq!(
            Vm::with_builtins()
                .build_env()
                .eval_str("(if true (* 10 2))")
                .unwrap(),
            vec![Val::Number(Number::Int(20))],
        )
    }

    #[test]
    fn if_with_false_and_single_arm_returns_void() {
        assert_eq!(
            Vm::with_builtins()
                .build_env()
                .eval_str("(if false (* 10 2))")
                .unwrap(),
            vec![Val::Void],
        )
    }

    #[test]
    fn recursive_function_definition_calls_recursively() {
        let mut env = Vm::with_builtins().build_env();
        assert_eq!(
            env.eval_str(
                r#"
(define fib (lambda (n) (if (<= n 2) 1 (+ (fib (- n 1)) (fib (- n 2))))))
(fib 10)
"#
            )
            .unwrap(),
            vec![Val::Void, Val::Number(Number::Int(55))],
        );
    }
}
