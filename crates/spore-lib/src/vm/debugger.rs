use std::rc::Rc;

use super::{
    environment::Environment,
    types::{proc::bytecode::ByteCodeProc, symbol::Symbol, Val},
};

pub trait Debugger {
    /// Called when a new procedure will be evaluated.
    fn start_eval(&mut self, _env: &Environment) {}

    /// Called when a procedure returns its value.
    fn return_value(&mut self, _val: &Val) {}

    /// Called when a new symbol is defined.
    fn define(&mut self, _env: &Environment, _sym: &Symbol, _val: &Val) {}
}

impl Debugger for () {}

/// Collects traces from an evaluation. The results can be printed by calling `.to_string()`.
///
/// Example output:
///
/// ```text
/// (<proc fib> 4) => 3
///   (<proc fib> 3) => 2
///     (<proc fib> 2) => 1
///     (<proc fib> 1) => 1
///   (<proc fib> 2) => 1
/// ```
#[derive(Default)]
pub struct TraceDebugger {
    traces: Vec<TraceCall>,
}

struct TraceCall {
    proc: Proc,
    args: Vec<Val>,
    return_val: Option<Val>,
    depth: usize,
}

enum Proc {
    ByteCode(Rc<ByteCodeProc>),
    Define,
}

impl TraceDebugger {
    /// Create a new `TraceDebugger`.
    pub fn new() -> TraceDebugger {
        Self::default()
    }
}

impl std::fmt::Display for TraceDebugger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut traces = self.traces.iter();
        match traces.next() {
            Some(t) => write!(f, "{t}")?,
            None => return write!(f, "_"),
        };
        for t in traces {
            write!(
                f,
                "
{t}"
            )?;
        }
        Ok(())
    }
}

impl std::fmt::Display for TraceCall {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for _ in 1..self.depth {
            write!(f, "  ")?;
        }
        match &self.proc {
            Proc::ByteCode(proc) => write!(f, "({proc}")?,
            Proc::Define => write!(f, "(define")?,
        };
        for arg in self.args.iter() {
            write!(f, " {arg}")?;
        }
        write!(f, ") => ")?;
        match self.return_val.as_ref() {
            Some(v) => write!(f, "{v}"),
            None => write!(f, "_"),
        }
    }
}

impl Debugger for TraceDebugger {
    fn start_eval(&mut self, env: &Environment) {
        if let Some(proc) = env.current_proc() {
            let args = env.frame_stack().to_vec();
            self.traces.push(TraceCall {
                proc: Proc::ByteCode(proc.clone()),
                args,
                return_val: None,
                depth: env.frame_depth(),
            })
        }
    }

    fn return_value(&mut self, val: &Val) {
        for t in self.traces.iter_mut().rev() {
            if t.return_val.is_none() {
                t.return_val = Some(val.clone());
                return;
            }
        }
    }

    fn define(&mut self, env: &Environment, sym: &Symbol, val: &Val) {
        let args = vec![sym.clone().into(), val.clone()];
        self.traces.push(TraceCall {
            proc: Proc::Define,
            args,
            return_val: Some(Val::Void),
            depth: env.frame_depth(),
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        parser::ast::Ast,
        vm::{
            ir::{CodeBlock, CodeBlockArgs},
            Vm,
        },
    };

    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn trace_prints_out_entire_trace() {
        let mut env = Vm::new().build_env();
        env.eval_str("(define (fib n) (if (<= n 2) 1 (+ (fib (- n 1)) (fib (- n 2)))))")
            .unwrap();
        let proc = {
            let ast: &Ast = &Ast::from_sexp_str("(fib 5)").unwrap()[0];
            let ir = CodeBlock::with_ast(CodeBlockArgs::default(), std::iter::once(ast)).unwrap();
            ir.to_bytecode()
        }
        .unwrap();
        let mut debugger = TraceDebugger::new();
        env.eval_bytecode(proc.into(), &[], &mut debugger).unwrap();
        assert_eq!(
            debugger.to_string(),
            r#"(<proc _>) => 5
  (<proc fib> 5) => 5
    (<proc fib> 4) => 3
      (<proc fib> 3) => 2
        (<proc fib> 2) => 1
        (<proc fib> 1) => 1
      (<proc fib> 2) => 1
    (<proc fib> 3) => 2
      (<proc fib> 2) => 1
      (<proc fib> 1) => 1"#
        );
    }

    #[test]
    fn error_encountered_in_stack_returns_trace_up_to_that_point() {
        let mut env = Vm::new().build_env();
        // This version of fib has a runtime error in its base case (when n <= 2).
        env.eval_str("(define (fib n) (if (<= n 2) (+ +) (+ (fib (- n 1)) (fib (- n 2)))))")
            .unwrap();
        let proc = {
            let ast: &Ast = &Ast::from_sexp_str("(fib 5)").unwrap()[0];
            let ir = CodeBlock::with_ast(CodeBlockArgs::default(), std::iter::once(ast)).unwrap();
            ir.to_bytecode()
        }
        .unwrap();
        let mut debugger = TraceDebugger::new();
        assert!(env.eval_bytecode(proc.into(), &[], &mut debugger).is_err());
        assert_eq!(
            debugger.to_string(),
            r#"(<proc _>) => _
  (<proc fib> 5) => _
    (<proc fib> 4) => _
      (<proc fib> 3) => _
        (<proc fib> 2) => _"#
        );
    }
}
