use std::sync::Arc;

use anyhow::Result;

use super::{instruction::Instruction, Val};

type NativeProcFn = Arc<dyn 'static + Send + Sync + Fn(&[Val]) -> Result<Val>>;

/// A function.
#[derive(Clone)]
pub enum Procedure {
    Native(&'static str, NativeProcFn),
    ByteCode(Arc<ByteCodeProc>),
}

/// A procedure that can be evaluated on an environment.
#[derive(Clone)]
pub struct ByteCodeProc {
    /// The name of the procedure.
    pub name: String,
    /// The number of arguments to the procedure.
    pub arg_count: usize,
    /// The bytecode to run.
    pub bytecode: Vec<Instruction>,
}

impl Procedure {
    /// Create a new function.
    pub fn with_native<P: 'static + Send + Sync + Fn(&[Val]) -> Result<Val>>(
        name: &'static str,
        proc: P,
    ) -> Arc<Procedure> {
        Arc::new(Procedure::Native(name, Arc::new(proc)))
    }

    pub fn with_bytecode(bc: Arc<ByteCodeProc>) -> Arc<Procedure> {
        Arc::new(Procedure::ByteCode(bc))
    }

    pub fn name(&self) -> &str {
        match self {
            Procedure::Native(name, _) => name,
            Procedure::ByteCode(bc) => match bc.name.as_str() {
                "" => "_",
                s => s,
            },
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

/// An iterator over bytecode.
pub struct ByteCodeIter {
    proc: Arc<ByteCodeProc>,
    next_idx: usize,
}

impl ByteCodeIter {
    /// Create an interator over a `ByteCodeProc` shared reference.
    pub fn from_proc(proc: Arc<ByteCodeProc>) -> ByteCodeIter {
        ByteCodeIter { proc, next_idx: 0 }
    }
}

impl ByteCodeIter {
    /// Jump some number of instructions.
    pub fn jump(&mut self, n: usize) {
        self.next_idx += n;
    }
}

impl Iterator for ByteCodeIter {
    type Item = Instruction;

    fn next(&mut self) -> Option<Instruction> {
        let res = self.proc.bytecode.get(self.next_idx).cloned();
        self.next_idx += 1;
        res
    }
}
