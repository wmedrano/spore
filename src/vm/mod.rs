use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock},
};

use anyhow::{bail, Result};

use crate::parser::ast::Ast;

use self::{
    bytecode::OpCode,
    compiler::ByteCodeProc,
    types::{GenericProcedure, Procedure, Symbol, Val},
};

pub mod bytecode;
pub mod compiler;
pub mod types;

type ValueRegistry = Arc<Mutex<HashMap<Symbol, Val>>>;

/// The spore virtual machine.
// Note: You typically use the global instance of the VM by calling / `Vm::singleton`.
#[derive(Clone)]
pub struct Vm {
    values: ValueRegistry,
    stack: Vec<Val>,
}

impl Vm {
    /// Get the global instance of the VM.
    pub fn singleton() -> Vm {
        static INITIALIZER: OnceLock<Vm> = OnceLock::new();
        let singleton = INITIALIZER.get_or_init(Self::with_builtins);
        singleton.clone()
    }

    /// Get a registered function.
    pub fn get_value(&self, sym: impl AsRef<str>) -> Option<Val> {
        let registry = self.values.lock().unwrap();
        registry.get(sym.as_ref()).cloned()
    }

    /// Register functions into the VM.
    pub fn register_global_fn(&self, fns: impl IntoIterator<Item = Arc<Procedure>>) {
        let mut registry = self.values.lock().unwrap();
        for f in fns {
            let name = f.name().unwrap().clone();
            let old_definition = registry.insert(name.clone(), Val::Proc(f));
            assert_eq!(
                old_definition,
                None,
                "Found duplicate definition for {name}.",
                name = name.as_ref(),
            );
        }
    }

    /// Register a value globally.
    pub fn register_global_value(&self, sym: Symbol, val: Val) -> Result<()> {
        let mut registry = self.values.lock().unwrap();
        if registry.contains_key(&sym) {
            bail!("symbol {sym} is already registered");
        }
        registry.insert(sym, val);
        Ok(())
    }

    /// Evaluate an sexpr and return the results as a vector.
    pub fn eval_sexpr(&mut self, s: &str) -> Result<Vec<Val>> {
        let asts = Ast::from_sexp_str(s)?;
        let mut res = Vec::new();
        for ast in asts {
            let bc = ByteCodeProc::with_ast(&ast)?;
            self.stack.clear();
            res.push(bc.eval(&mut self.stack, 0)?);
        }
        Ok(res)
    }

    /// Evaluate a sequence of bytecode.
    pub fn eval_bytecode(&self, bc: &[OpCode], stack: &mut Vec<Val>) -> Result<()> {
        let mut iter = bc.iter();
        while let Some(bc) = iter.next() {
            match bc {
                OpCode::PushVal(v) => self.push_value(v, stack),
                OpCode::Eval(n) => self.eval_n(*n, stack)?,
                OpCode::JumpIf(n) => self.jump_if(*n, stack, &mut iter)?,
                OpCode::Jump(n) => self.jump(*n, &mut iter),
            }
        }
        Ok(())
    }

    /// We disable inlining to preserve debug symbols for profiling.
    #[inline(never)]
    fn jump_if<'a>(
        &self,
        n: usize,
        stack: &mut Vec<Val>,
        bc_iter: &mut impl Iterator<Item = &'a OpCode>,
    ) -> Result<()> {
        match stack.pop() {
            Some(Val::Bool(false)) => (),
            Some(_) => {
                bc_iter.nth(n - 1);
            }
            None => bail!("bytecode if found no value to evaluate if statement"),
        }
        Ok(())
    }

    /// We disable inlining to preserve debug symbols for profiling.
    #[inline(never)]
    fn jump<'a>(&self, n: usize, bc_iter: &mut impl Iterator<Item = &'a OpCode>) {
        bc_iter.nth(n - 1);
    }

    /// We disable inlining to preserve debug symbols for profiling.
    #[inline(never)]
    fn push_value(&self, v: &Val, stack: &mut Vec<Val>) {
        stack.push(v.clone());
    }

    /// We disable inlining to preserve debug symbols for profiling.
    #[inline(never)]
    fn eval_n(&self, n: usize, stack: &mut Vec<Val>) -> Result<()> {
        let proc_idx = stack.len() - n;
        let arg_count = n - 1;
        let proc = match stack.get(proc_idx) {
            None => bail!("eval_top must have at least one value on the stack"),
            Some(Val::Proc(p)) => p.clone(),
            Some(v) => bail!("value {v} is not a valid procedure."),
        };
        let val = proc.eval(stack, arg_count)?;
        *stack.last_mut().unwrap() = val;
        Ok(())
    }

    /// Create a new `Vm` with all the builtins.
    fn with_builtins() -> Vm {
        let vm = Vm {
            values: ValueRegistry::new(Mutex::new(HashMap::new())),
            stack: Vec::new(),
        };
        crate::builtins::register_all(&vm);
        vm
    }
}

#[cfg(test)]
mod tests {
    use crate::vm::types::{Number, Symbol};
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn can_execute_instructions() {
        let vm = Vm::singleton();
        let mut result = Vec::new();
        vm.eval_bytecode(
            &[
                OpCode::PushVal(vm.get_value(&Symbol::from("+")).unwrap()),
                OpCode::PushVal(Val::Number(Number::Int(10))),
                OpCode::PushVal(Val::Number(Number::Int(5))),
                OpCode::PushVal(Val::Number(Number::Int(3))),
                OpCode::Eval(4),
            ],
            &mut result,
        )
        .unwrap();
        assert_eq!(result, &[Val::Number(Number::Int(18))]);
    }

    #[test]
    fn can_execute_ast() {
        let result = Vm::singleton().eval_sexpr("(+ 1 2 (- 3 4))").unwrap();
        assert_eq!(result, &[Val::Number(Number::Int(2))])
    }

    #[test]
    fn if_with_true_returns_first_expr_result() {
        let result = Vm::singleton()
            .eval_sexpr("(if true (* 10 2) (+ 10 2))")
            .unwrap();
        assert_eq!(result, &[Val::Number(Number::Int(20))])
    }

    #[test]
    fn if_with_false_returns_second_expr_result() {
        let result = Vm::singleton()
            .eval_sexpr("(if false (* 10 2) (+ 10 2))")
            .unwrap();
        assert_eq!(result, &[Val::Number(Number::Int(12))])
    }

    #[test]
    fn if_with_true_and_single_arm_returns_true() {
        let result = Vm::singleton().eval_sexpr("(if true (* 10 2))").unwrap();
        assert_eq!(result, &[Val::Number(Number::Int(20))])
    }

    #[test]
    fn if_with_false_and_single_arm_returns_void() {
        let result = Vm::singleton().eval_sexpr("(if false (* 10 2))").unwrap();
        assert_eq!(result, &[Val::Void])
    }
}
