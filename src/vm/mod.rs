use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock},
};

use anyhow::{bail, Result};

use crate::parser::ast::Ast;

use self::{
    bytecode::ByteCode,
    types::{Procedure, Symbol, Val},
};

pub mod bytecode;
pub mod types;

type ValueRegistry = Mutex<HashMap<Symbol, Val>>;

/// The spore virtual machine.
// Note: You typically use the global instance of the VM by calling / `Vm::singleton`.
pub struct Vm {
    values: ValueRegistry,
}

impl Vm {
    /// Get the global instance of the VM.
    pub fn singleton() -> &'static Vm {
        static INITIALIZER: OnceLock<Vm> = OnceLock::new();
        INITIALIZER.get_or_init(Self::with_builtins)
    }

    /// Get a registered function.
    pub fn get_value(&self, sym: impl AsRef<str>) -> Option<Val> {
        let registry = self.values.lock().unwrap();
        registry.get(sym.as_ref()).cloned()
    }

    /// Register functions into the VM.
    pub fn register_fns(&self, fns: impl Iterator<Item = Arc<Procedure>>) {
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
    pub fn register_value(&self, sym: Symbol, val: Val) -> Result<()> {
        let mut registry = self.values.lock().unwrap();
        if registry.contains_key(&sym) {
            bail!("symbol {sym} is already registered");
        }
        registry.insert(sym, val);
        Ok(())
    }

    /// Evaluate an sexpr and return the results as a vector.
    pub fn eval_sexpr(&self, s: &str) -> Result<Vec<Val>> {
        let mut res = Vec::new();
        let asts = Ast::from_sexp_str(s)?;
        for ast in asts {
            res = self.eval_bytecode(&ByteCode::with_ast(&ast)?, res)?;
        }
        Ok(res)
    }

    /// Evaluate a sequence of bytecode.
    pub fn eval_bytecode(&self, bc: &[ByteCode], stack: Vec<Val>) -> Result<Vec<Val>> {
        let mut iter = bc.iter();
        let mut stack = stack;
        while let Some(bc) = iter.next() {
            match bc {
                ByteCode::PushVal(v) => self.push_value(v, &mut stack),
                ByteCode::Eval(n) => self.eval_n(*n, &mut stack)?,
                ByteCode::JumpIf(n) => self.jump_if(*n, &mut stack, &mut iter)?,
                ByteCode::Jump(n) => self.jump(*n, &mut iter),
            }
        }
        Ok(stack)
    }

    /// We disable inlining to preserve debug symbols for profiling.
    #[inline(never)]
    fn jump_if<'a>(
        &self,
        n: usize,
        stack: &mut Vec<Val>,
        bc_iter: &mut impl Iterator<Item = &'a ByteCode>,
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
    fn jump<'a>(&self, n: usize, bc_iter: &mut impl Iterator<Item = &'a ByteCode>) {
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
        let res = {
            let mut stack_values = stack.drain(stack.len() - n..);
            let proc = match stack_values.next() {
                None => bail!("eval_top must have at least one value on the stack"),
                Some(Val::Proc(p)) => p,
                Some(v) => bail!("value {v} is not a valid procedure."),
            };
            proc.eval(stack_values.as_slice())?
        };
        stack.push(res);
        Ok(())
    }

    /// Create a new `Vm` with all the builtins.
    fn with_builtins() -> Vm {
        let vm = Vm {
            values: ValueRegistry::new(HashMap::new()),
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
        let result = vm
            .eval_bytecode(
                &[
                    ByteCode::PushVal(vm.get_value(&Symbol::from("+")).unwrap()),
                    ByteCode::PushVal(Val::Number(Number::Int(10))),
                    ByteCode::PushVal(Val::Number(Number::Int(5))),
                    ByteCode::PushVal(Val::Number(Number::Int(3))),
                    ByteCode::Eval(4),
                ],
                Vec::new(),
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
