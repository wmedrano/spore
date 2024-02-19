use std::sync::Arc;

use anyhow::Result;

use crate::vm::compiler::ByteCodeProc;

use super::Val;

type NativeProcFn = Box<dyn 'static + Send + Sync + Fn(&[Val]) -> Result<Val>>;

/// A function.
pub enum Procedure {
    Native(&'static str, NativeProcFn),
    ByteCode(ByteCodeProc),
}

impl Procedure {
    /// Create a new function.
    pub fn with_native<P: 'static + Send + Sync + Fn(&[Val]) -> Result<Val>>(
        name: &'static str,
        proc: P,
    ) -> Arc<Procedure> {
        Arc::new(Procedure::Native(name, Box::new(proc)))
    }

    pub fn with_bytecode(bc: ByteCodeProc) -> Arc<Procedure> {
        Arc::new(Procedure::ByteCode(bc))
    }

    pub fn name(&self) -> &str {
        match self {
            Procedure::Native(name, _) => name,
            Procedure::ByteCode(_) => "_",
        }
    }
}

impl PartialEq for Procedure {
    fn eq(&self, _: &Self) -> bool {
        false
    }
}

impl std::fmt::Debug for Procedure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = self.name();
        f.debug_struct("Procedure")
            .field("name", &name)
            .finish_non_exhaustive()
    }
}

impl std::fmt::Display for Procedure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<proc {name}>", name = self.name())
    }
}
