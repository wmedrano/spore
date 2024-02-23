use std::{collections::HashMap, sync::Arc};

use anyhow::Result;

use self::{
    environment::Environment,
    types::{proc::Procedure, symbol::Symbol, Val},
};

pub mod compiler;
pub mod environment;
pub mod types;

type ValueRegistry = HashMap<Symbol, Val>;

/// The spore virtual machine.
// Note: You typically use the global instance of the VM by calling / `Vm::with_builtins`.
#[derive(Clone)]
pub struct Vm {
    globals: ValueRegistry,
}

impl Vm {
    /// Create a new `Vm` with all the builtins.
    pub fn with_builtins() -> Vm {
        let mut vm = Vm {
            globals: ValueRegistry::new(),
        };
        crate::builtins::register_all(&mut vm);
        vm
    }

    /// Register functions into the VM.
    pub fn register_global_fn(
        &mut self,
        fns: impl IntoIterator<Item = Arc<Procedure>>,
    ) -> Result<()> {
        for f in fns {
            let sym = Symbol::from(f.name());
            self.register_global_value(sym, Val::Proc(f))?;
        }
        Ok(())
    }

    /// Register a value globally.
    pub fn register_global_value(&mut self, sym: Symbol, val: Val) -> Result<()> {
        let _old_value = self.globals.insert(sym, val);
        Ok(())
    }

    /// Create a new environment that can evaluate bytecode.
    pub fn build_env(&self) -> Environment {
        Environment {
            globals: self.globals.clone(),
            stack: Vec::with_capacity(4096),
            frames: Vec::with_capacity(64),
        }
    }
}
