use std::rc::Rc;

use anyhow::Result;

use crate::vm::types::Val;

type NativeProcFn = dyn 'static + Send + Sync + Fn(&[Val]) -> Result<Val>;

#[derive(Clone)]
pub struct NativeProc {
    name: &'static str,
    f: Rc<NativeProcFn>,
}

impl NativeProc {
    /// Create a new native procedure from a Rust function.
    pub fn new<P: 'static + Send + Sync + Fn(&[Val]) -> Result<Val>>(
        name: &'static str,
        proc: P,
    ) -> Rc<NativeProc> {
        let f = Rc::new(proc);
        Rc::new(NativeProc { name, f })
    }

    /// Get the name of the native procedure.
    pub fn name(&self) -> &str {
        self.name
    }

    /// Evaluate the native procedure.
    pub fn eval(&self, args: &[Val]) -> Result<Val> {
        (self.f)(args)
    }
}

impl PartialEq for NativeProc {
    fn eq(&self, _: &Self) -> bool {
        false
    }
}

impl std::fmt::Debug for NativeProc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Procedure")
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

impl std::fmt::Display for NativeProc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{proc {name}}}", name = &self.name)
    }
}
