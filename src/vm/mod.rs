use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock},
};

use anyhow::{bail, Result};

use crate::parser::ast::Ast;

use self::{
    compiler::Compiler,
    environment::{Environment, LocalEnvironment},
    types::{proc::Procedure, symbol::Symbol, Val},
};

pub mod compiler;
pub mod environment;
pub mod types;

type ValueRegistry = Arc<Mutex<HashMap<Symbol, Val>>>;

/// The spore virtual machine.
// Note: You typically use the global instance of the VM by calling / `Vm::singleton`.
#[derive(Clone)]
pub struct Vm {
    globals: ValueRegistry,
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
        let registry = self.globals.lock().unwrap();
        registry.get(sym.as_ref()).cloned()
    }

    /// Register functions into the VM.
    pub fn register_global_fn(&self, fns: impl IntoIterator<Item = Arc<Procedure>>) {
        let mut registry = self.globals.lock().unwrap();
        for f in fns {
            let name = f.name().to_string();
            let old_definition = registry.insert(Symbol::from(name.clone()), Val::Proc(f));
            assert_eq!(
                old_definition, None,
                "Found duplicate definition for {name}.",
            );
        }
    }

    /// Register a value globally.
    pub fn register_global_value(&self, sym: Symbol, val: Val) -> Result<()> {
        let mut registry = self.globals.lock().unwrap();
        if registry.contains_key(&sym) {
            bail!("symbol {sym} is already registered");
        }
        registry.insert(sym, val);
        Ok(())
    }

    /// Create a new environment that can evaluate bytecode.
    pub fn env(&self) -> Environment {
        Environment {
            globals: self.globals.clone(),
            stack: Vec::with_capacity(4096),
            local: LocalEnvironment { stack_base: 0 },
        }
    }

    /// Evaluate an sexpr and return the results as a vector.
    pub fn eval_sexpr<T: AsRef<str>>(&mut self, expr: T) -> Result<Vec<Val>> {
        let asts = Ast::from_sexp_str(expr.as_ref())?;
        let mut res = Vec::with_capacity(asts.len());
        let mut env = self.env();
        for ast in asts {
            let bc = Compiler::new().compile_and_finalize(&ast)?;
            let val = bc.eval(&mut env)?;
            env.reset_locals();
            res.push(val);
        }
        Ok(res)
    }

    /// Create a new `Vm` with all the builtins.
    fn with_builtins() -> Vm {
        let vm = Vm {
            globals: ValueRegistry::new(Mutex::new(HashMap::new())),
        };
        crate::builtins::register_all(&vm);
        vm
    }
}

#[cfg(test)]
mod tests {
    use crate::vm::types::Number;
    use pretty_assertions::assert_eq;

    use super::*;

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

    #[test]
    fn recursive_function_definition_calls_recursively() {
        let result = Vm::singleton()
            .eval_sexpr(
                vec![
                    "(def fib (lambda (n) (if (<= n 2) 1 (+ (fib (- n 1)) (fib (- n 2))))))",
                    "(fib 10)",
                ]
                .join("\n"),
            )
            .unwrap();
        assert_eq!(result, &[Val::Void, Val::Number(Number::Int(55))]);
    }
}
