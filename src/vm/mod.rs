use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock},
};

use self::{
    expr::Expr,
    types::{Function, Symbol, Val},
};

mod expr;
pub mod types;

type ValueRegistry = Mutex<HashMap<Symbol, Val>>;

/// The spore virtual machine.
// Note: You typically use the global instance of the VM by calling / `Vm::singleton`.
pub struct Vm {
    functions: ValueRegistry,
}

impl Vm {
    /// Get the global instance of the VM.
    pub fn singleton() -> &'static Vm {
        static INITIALIZER: OnceLock<Vm> = OnceLock::new();
        INITIALIZER.get_or_init(Self::with_builtins)
    }

    /// Evaluate a string and return the values of the resulting expressions.
    pub fn eval_str(s: &str) -> Vec<Val> {
        Expr::eval_str(s)
    }

    /// Get a registered function.
    pub fn get_value(&self, f: &Symbol) -> Option<Val> {
        let registry = self.functions.lock().unwrap();
        registry.get(f).cloned()
    }

    /// Register functions into the VM.
    pub fn register_fns(&self, fns: impl Iterator<Item = (Symbol, Arc<Function>)>) {
        let mut registry = self.functions.lock().unwrap();
        for (name, f) in fns {
            let old_definition = registry.insert(name.clone(), Val::Function(f));
            assert_eq!(
                old_definition,
                None,
                "Found duplicate definition for {name}.",
                name = name.as_str(),
            );
        }
    }

    /// Create a new `Vm` with all the builtins.
    fn with_builtins() -> Vm {
        let vm = Vm {
            functions: ValueRegistry::new(HashMap::new()),
        };
        crate::builtins::register_all(&vm);
        vm
    }
}
