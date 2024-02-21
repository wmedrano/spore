use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock},
};

use anyhow::Result;

use self::{
    environment::Environment,
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
    pub fn register_global_fn(&self, fns: impl IntoIterator<Item = Arc<Procedure>>) -> Result<()> {
        for f in fns {
            let sym = Symbol::from(f.name());
            self.register_global_value(sym, Val::Proc(f))?;
        }
        Ok(())
    }

    /// Register a value globally.
    pub fn register_global_value(&self, sym: Symbol, val: Val) -> Result<()> {
        let mut registry = self.globals.lock().unwrap();
        let _old_value = registry.insert(sym, val);
        Ok(())
    }

    /// Create a new environment that can evaluate bytecode.
    pub fn env(&self) -> Environment {
        Environment {
            globals: self.globals.clone(),
            stack: Vec::with_capacity(4096),
            frames: Vec::with_capacity(64),
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
