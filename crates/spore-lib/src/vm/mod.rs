use std::rc::Rc;

use anyhow::Result;

use self::{
    environment::Environment,
    module::{Module, ModuleSource},
    types::{proc::native::NativeProc, symbol::Symbol, Val},
};

pub mod debugger;
pub mod environment;
pub mod ir;
pub mod module;
pub mod types;

/// The spore virtual machine.
// Note: You typically use the global instance of the VM by calling / `Vm::with_builtins`.
#[derive(Clone)]
pub struct Vm {
    globals: Module,
}

impl Default for Vm {
    fn default() -> Vm {
        Vm::new()
    }
}

impl Vm {
    /// Create a new `Vm` with all the builtins.
    pub fn new() -> Vm {
        let mut vm = Vm {
            globals: Module::new(ModuleSource::Global),
        };
        crate::builtins::register_all(&mut vm);
        vm
    }

    /// Register functions into the VM.
    pub fn register_global_procs(
        &mut self,
        fns: impl IntoIterator<Item = Rc<NativeProc>>,
    ) -> Result<()> {
        for f in fns {
            let sym = Symbol::from(f.name());
            self.register_global_value(sym, Val::NativeProc(f))?;
        }
        Ok(())
    }

    /// Register a value globally.
    pub fn register_global_value(&mut self, sym: Symbol, val: Val) -> Result<()> {
        self.globals.set(sym, val);
        Ok(())
    }

    /// Create a new environment that can evaluate bytecode.
    pub fn build_env(&self) -> Environment {
        Environment::new(self)
    }
}
