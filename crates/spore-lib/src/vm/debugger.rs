use std::rc::Rc;

use super::{
    types::{proc::bytecode::ByteCodeProc, symbol::Symbol, Val},
    Vm,
};

pub trait Debugger {
    /// Called when a new procedure will be evaluated. The new procedure will be the top frame in
    /// the `Vm`.
    fn eval_proc(&mut self, _vm: &Vm) {}

    /// Called when a procedure returns its value.
    fn return_value(&mut self, _val: &Val) {}

    /// Called when a new symbol is defined.
    fn define(&mut self, _vm: &Vm, _sym: &Symbol, _val: &Val) {}
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
    fn eval_proc(&mut self, vm: &Vm) {
        if let Some(proc) = vm.current_proc() {
            let args = vm.frame_stack().to_vec();
            self.traces.push(TraceCall {
                proc: Proc::ByteCode(proc.clone()),
                args,
                return_val: None,
                depth: vm.frame_depth(),
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

    fn define(&mut self, vm: &Vm, sym: &Symbol, val: &Val) {
        let args = vec![sym.clone().into(), val.clone()];
        self.traces.push(TraceCall {
            proc: Proc::Define,
            args,
            return_val: Some(Val::Void),
            depth: vm.frame_depth(),
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        parser::ast::Ast,
        vm::{
            ir::{CodeBlock, CodeBlockArgs},
            module::ModuleSource,
        },
    };

    use super::*;
    use pretty_assertions::assert_eq;

    const MODULE: ModuleSource = ModuleSource::Virtual("test");

    #[test]
    fn trace_prints_out_entire_trace() {
        let mut vm = Vm::new();
        vm.eval_str(
            MODULE,
            "(define (fib n) (if (<= n 2) 1 (+ (fib (- n 1)) (fib (- n 2)))))",
        )
        .unwrap();
        let proc = {
            let ast: &Ast = &Ast::from_sexp_str("(fib 5)").unwrap()[0];
            let ir = CodeBlock::with_ast(CodeBlockArgs::default(), std::iter::once(ast)).unwrap();
            ir.to_proc(MODULE)
        }
        .unwrap();
        let mut debugger = TraceDebugger::new();
        vm.eval_bytecode(proc.into(), &[], &mut debugger).unwrap();
        assert_eq!(
            debugger.to_string(),
            r#"(<proc _>) => 5
  (<proc fib> 5) => 5
    (<proc <=> 5 2) => false
    (<proc -> 5 1) => 4
    (<proc fib> 4) => 3
      (<proc <=> 4 2) => false
      (<proc -> 4 1) => 3
      (<proc fib> 3) => 2
        (<proc <=> 3 2) => false
        (<proc -> 3 1) => 2
        (<proc fib> 2) => 1
          (<proc <=> 2 2) => true
        (<proc -> 3 2) => 1
        (<proc fib> 1) => 1
          (<proc <=> 1 2) => true
        (<proc +> 1 1) => 2
      (<proc -> 4 2) => 2
      (<proc fib> 2) => 1
        (<proc <=> 2 2) => true
      (<proc +> 2 1) => 3
    (<proc -> 5 2) => 3
    (<proc fib> 3) => 2
      (<proc <=> 3 2) => false
      (<proc -> 3 1) => 2
      (<proc fib> 2) => 1
        (<proc <=> 2 2) => true
      (<proc -> 3 2) => 1
      (<proc fib> 1) => 1
        (<proc <=> 1 2) => true
      (<proc +> 1 1) => 2
    (<proc +> 3 2) => 5"#
        );
    }

    #[test]
    fn error_encountered_in_stack_returns_trace_up_to_that_point() {
        let mut vm = Vm::new();
        // This version of fib has a runtime error in its base case (when n <= 2).
        vm.eval_str(
            MODULE,
            "(define (fib n) (if (<= n 2) (+ +) (+ (fib (- n 1)) (fib (- n 2)))))",
        )
        .unwrap();
        let ast: &Ast = &Ast::from_sexp_str("(fib 5)").unwrap()[0];
        let ir = CodeBlock::with_ast(CodeBlockArgs::default(), std::iter::once(ast)).unwrap();
        let proc = ir.to_proc(MODULE).unwrap();
        let mut debugger = TraceDebugger::new();
        assert!(vm.eval_bytecode(proc.into(), &[], &mut debugger).is_err());
        assert_eq!(
            debugger.to_string(),
            r#"(<proc _>) => _
  (<proc fib> 5) => _
    (<proc <=> 5 2) => false
    (<proc -> 5 1) => 4
    (<proc fib> 4) => _
      (<proc <=> 4 2) => false
      (<proc -> 4 1) => 3
      (<proc fib> 3) => _
        (<proc <=> 3 2) => false
        (<proc -> 3 1) => 2
        (<proc fib> 2) => _
          (<proc <=> 2 2) => true
          (<proc +> <proc +>) => _"#,
        );
    }
}
