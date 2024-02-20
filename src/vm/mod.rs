use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock},
};

use anyhow::{bail, Result};

use self::{
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

    /// Create a new `Vm` with all the builtins.
    fn with_builtins() -> Vm {
        let vm = Vm {
            globals: ValueRegistry::new(Mutex::new(HashMap::new())),
        };
        crate::builtins::register_all(&vm);
        vm
    }
}
