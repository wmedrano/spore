use std::rc::Rc;

use anyhow::Result;
use smol_str::SmolStr;

use crate::vm::{
    module::{ModuleManager, ModuleSource},
    types::Val,
};

use super::bytecode::{ByteCodeIter, ByteCodeProc};

type NativeProcFn = dyn 'static + Send + Sync + Fn(&ModuleManager, &[Val]) -> Result<Val>;

pub struct NativeProc {
    placeholder_bytecode: Rc<ByteCodeProc>,
    f: Box<NativeProcFn>,
}

impl NativeProc {
    /// Create a new native procedure from a Rust function.
    pub fn new<P: 'static + Send + Sync + Fn(&ModuleManager, &[Val]) -> Result<Val>>(
        name: &'static str,
        proc: P,
    ) -> Rc<NativeProc> {
        let f = Box::new(proc);
        let placeholder_bytecode = Rc::new(ByteCodeProc {
            name: SmolStr::new_static(name),
            arg_count: 0,
            bytecode: Vec::new(),
            module: ModuleSource::Global,
            is_module_definition: false,
        });
        Rc::new(NativeProc {
            placeholder_bytecode,
            f,
        })
    }

    pub fn placeholder_bytecode_iter(&self) -> ByteCodeIter {
        ByteCodeIter::from_proc(self.placeholder_bytecode.clone())
    }

    /// Get the name of the native procedure.
    pub fn name(&self) -> &str {
        self.placeholder_bytecode.name.as_str()
    }

    /// Evaluate the native procedure.
    pub fn eval(&self, modules: &ModuleManager, args: &[Val]) -> Result<Val> {
        (self.f)(modules, args)
    }
}

impl PartialEq for NativeProc {
    fn eq(&self, other: &Self) -> bool {
        let ptr = self.f.as_ref() as *const _;
        let other = other.f.as_ref() as *const _;
        std::ptr::addr_eq(ptr, other)
    }
}

impl std::fmt::Debug for NativeProc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Procedure")
            .field("name", &self.name())
            .field("module", &self.placeholder_bytecode.module)
            .finish_non_exhaustive()
    }
}

impl std::fmt::Display for NativeProc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<proc {name}>", name = &self.name())
    }
}
