use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock},
};

use self::types::{Procedure, Symbol, Val};

pub mod expr;
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

    /// Get a registered function.
    pub fn get_value(&self, f: &Symbol) -> Option<Val> {
        let registry = self.functions.lock().unwrap();
        registry.get(f).cloned()
    }

    /// Register functions into the VM.
    pub fn register_fns(&self, fns: impl Iterator<Item = Arc<Procedure>>) {
        let mut registry = self.functions.lock().unwrap();
        for f in fns {
            let name = f.name().unwrap().clone();
            let old_definition = registry.insert(name.clone(), Val::Proc(f));
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
