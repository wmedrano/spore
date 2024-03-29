use std::{collections::HashMap, rc::Rc};

use anyhow::{bail, Context, Result};

use crate::parser::ast::Ast;

use super::{
    compiler::Compiler,
    debugger::Debugger,
    types::{
        instruction::Instruction,
        proc::bytecode::{ByteCodeIter, ByteCodeProc},
        symbol::Symbol,
        Val,
    },
    Vm,
};

/// An environment to evaluate bytecode on.
pub struct Environment {
    /// The registry of global values.
    globals: HashMap<Symbol, Val>,
    /// The processing stack.
    stack: Vec<Val>,
    /// Contains the current call frame. This includes the instructions that should be run and the
    /// base of the frame's stack.
    frames: Vec<Frame>,
}

pub struct Frame {
    /// The bytecode that is under execution in the frame.
    bytecode: ByteCodeIter,
    /// The index of the local stack.
    stack_start_idx: usize,
}

struct StackTrace {
    trace: Vec<String>,
}

impl Environment {
    /// Create a new environment.
    pub fn new(vm: &Vm) -> Environment {
        Environment {
            globals: vm.globals.clone(),
            stack: Vec::with_capacity(4096),
            frames: Vec::with_capacity(128),
        }
    }

    /// Evaluate an S-Expression string and return the last value. If there are no expression, than
    /// `Val::Void` is returned.
    pub fn eval_str(&mut self, s: &str) -> Result<Vec<Val>> {
        Ast::from_sexp_str(s)?
            .into_iter()
            .map(|ast| {
                let proc = Compiler::new("eval", self).compile(&ast)?;
                self.eval_bytecode(proc.into(), &[])
            })
            .collect()
    }

    /// Evaluate a bytecode procedure with the given arguments.
    pub fn eval_bytecode(&mut self, proc: Rc<ByteCodeProc>, args: &[Val]) -> Result<Val> {
        self.eval_bytecode_impl(proc, args, &mut ())
            .with_context(|| self.stack_trace())
    }

    /// Evaluate a bytecode procedure with the given arguments and the given debugger.
    pub fn eval_with_debugger(
        &mut self,
        proc: Rc<ByteCodeProc>,
        args: &[Val],
        debugger: &mut impl Debugger,
    ) -> Result<Val> {
        self.eval_bytecode_impl(proc, args, debugger)
            .with_context(|| self.stack_trace())
    }

    /// Set a symbol to a global value.
    pub fn set_global(&mut self, sym: Symbol, val: Val) {
        // TODO: Consider signaling when a symbol is being overwritten.
        self.globals.insert(sym, val);
    }

    /// Gets the value of a global symbol or `None` if it is not defined.
    pub fn get_global(&self, sym: &Symbol) -> Option<Val> {
        self.globals.get(sym).cloned()
    }

    /// The values on the current stack frame.
    pub fn frame_stack(&self) -> &[Val] {
        match self.frames.last() {
            None => &[],
            Some(f) => &self.stack[f.stack_start_idx..],
        }
    }

    /// The current depth of the call stack.
    pub fn frame_depth(&self) -> usize {
        self.frames.len()
    }

    /// Get the current procedure.
    pub fn current_proc(&self) -> Option<&Rc<ByteCodeProc>> {
        self.frames.last().map(|f| f.bytecode.inner())
    }

    /// Evaluate a sequence of bytecode.
    fn eval_bytecode_impl(
        &mut self,
        proc: Rc<ByteCodeProc>,
        args: &[Val],
        debugger: &mut impl Debugger,
    ) -> Result<Val> {
        self.prepare(proc, args);
        debugger.start_eval(self);
        while let Some(frame) = self.frames.last_mut() {
            let instruction = frame.bytecode.next_instruction();
            match instruction {
                Instruction::PushVal(v) => {
                    let v = v.clone();
                    self.execute_push_val(v);
                }
                Instruction::Eval(n) => {
                    let n = *n;
                    self.execute_eval_n(n, debugger)?
                }
                Instruction::GetArg(n) => {
                    let n = *n;
                    self.execute_get_arg(n)
                }
                Instruction::GetVal(s) => match self.globals.get(s) {
                    Some(v) => {
                        self.execute_push_val(v.clone());
                    }
                    None => bail!("{s} is not defined"),
                },
                Instruction::JumpIf(n) => {
                    let n = *n;
                    self.execute_jump_if(n)?
                }
                Instruction::Jump(n) => {
                    let n = *n;
                    self.execute_jump(n)
                }
                Instruction::Return => {
                    self.pop_frame(debugger)?;
                }
                Instruction::SetVal(s) => {
                    let s = s.clone();
                    self.execute_set_val(s, debugger)?
                }
            }
        }
        let ret = self.stack.pop().unwrap_or_default();
        debugger.return_value(&ret);
        Ok(ret)
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

    #[cold]
    fn stack_trace(&self) -> StackTrace {
        StackTrace {
            trace: self
                .frames
                .iter()
                .map(|f| f.bytecode.inner().name.clone())
                .collect(),
        }
    }

    /// Pop the current frame. This truncates the local stack and replaces the top value of the
    /// stack with the return value. The return value is defined as the value at the top of the
    /// local stack.
    fn pop_frame(&mut self, debugger: &mut impl Debugger) -> Result<()> {
        let frame = self.frames.pop().unwrap();
        let return_val = if self.stack.len() > frame.stack_start_idx {
            self.stack.pop().unwrap_or_default()
        } else {
            Val::Void
        };
        debugger.return_value(&return_val);
        self.stack.truncate(frame.stack_start_idx);
        self.stack.pop();
        self.stack.push(return_val);
        Ok(())
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

    fn execute_eval_n(&mut self, n: usize, debugger: &mut impl Debugger) -> Result<()> {
        let proc_idx = self.stack.len() - n;
        let proc_val = std::mem::take(&mut self.stack[proc_idx]);
        match proc_val {
            Val::ByteCodeProc(proc) => {
                let expected_args = proc.arg_count;
                let actual_args = n - 1;
                self.frames.push(Frame {
                    bytecode: ByteCodeIter::from_proc(proc),
                    stack_start_idx: proc_idx + 1,
                });
                debugger.start_eval(self);
                if expected_args != actual_args {
                    bail!(
                        "{name} expected {expected_args} but found {actual_args}",
                        name = self.current_proc().map(|p| p.name.as_str()).unwrap_or("_")
                    );
                }
            }
            Val::NativeProc(proc) => {
                let stack_base = proc_idx + 1;
                let res = {
                    let args = self.stack.drain(stack_base..);
                    proc.eval(args.as_slice())?
                };
                *self.stack.last_mut().unwrap() = res;
            }
            v => bail!("expected procedure but found {v}"),
        };
        Ok(())
    }

    fn execute_push_val(&mut self, val: Val) {
        self.stack.push(val);
    }

    fn execute_set_val(&mut self, s: Symbol, debugger: &mut impl Debugger) -> Result<()> {
        let v = self.stack.pop().unwrap();
        debugger.define(self, &s, &v);
        self.globals.insert(s, v);
        Ok(())
    }
}

impl std::fmt::Display for StackTrace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Stack trace:")?;
        for trace in self.trace.iter() {
            writeln!(f, "  {}", trace)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::vm::Vm;

    use super::*;

    #[test]
    fn can_execute_ast() {
        assert_eq!(
            Vm::new().build_env().eval_str("(+ 1 2 (- 3 4))").unwrap(),
            vec![2.into()]
        );
    }

    #[test]
    fn if_with_true_returns_first_expr_result() {
        assert_eq!(
            Vm::new()
                .build_env()
                .eval_str("(if true (* 10 2) (+ 10 2))")
                .unwrap(),
            vec![20.into()],
        );
    }

    #[test]
    fn if_with_false_returns_second_expr_result() {
        assert_eq!(
            Vm::new()
                .build_env()
                .eval_str("(if false (* 10 2) (+ 10 2))")
                .unwrap(),
            vec![12.into()],
        )
    }

    #[test]
    fn if_with_true_and_single_arm_returns_true() {
        assert_eq!(
            Vm::new()
                .build_env()
                .eval_str("(if true (* 10 2))")
                .unwrap(),
            vec![20.into()],
        )
    }

    #[test]
    fn if_with_false_and_single_arm_returns_void() {
        assert_eq!(
            Vm::new()
                .build_env()
                .eval_str("(if false (* 10 2))")
                .unwrap(),
            vec![Val::Void],
        )
    }

    #[test]
    fn recursive_function_definition_calls_recursively() {
        let mut env = Vm::new().build_env();
        assert_eq!(
            env.eval_str(
                r#"
(define (fib n) (if (<= n 2) 1 (+ (fib (- n 1)) (fib (- n 2)))))
(fib 10)
"#
            )
            .unwrap(),
            vec![Val::Void, 55.into()],
        );
    }
}
