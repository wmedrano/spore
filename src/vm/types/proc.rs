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
#[derive(Clone)]
pub struct ByteCodeIter {
    _proc: Arc<ByteCodeProc>,
    next_ptr: *const Instruction,
    end_ptr: *const Instruction,
}

impl ByteCodeIter {
    /// Create an interator over a `ByteCodeProc` shared reference.
    pub fn from_proc(proc: Arc<ByteCodeProc>) -> ByteCodeIter {
        let next = proc.bytecode.as_ptr();
        let end = unsafe { next.offset(proc.bytecode.len() as isize) };
        ByteCodeIter {
            _proc: proc,
            next_ptr: next,
            end_ptr: end,
        }
    }
}

impl ByteCodeIter {
    /// Jump some number of instructions.
    pub fn jump(&mut self, n: usize) {
        self.next_ptr = unsafe { self.next_ptr.offset(n as isize).min(self.end_ptr) };
    }
}

impl Iterator for ByteCodeIter {
    type Item = Instruction;

    fn next(&mut self) -> Option<Instruction> {
        if self.next_ptr < self.end_ptr {
            let res = (unsafe { &*self.next_ptr }).clone();
            self.next_ptr = unsafe { self.next_ptr.offset(1) };
            Some(res)
        } else {
            None
        }
    }

    fn count(self) -> usize {
        let offset = unsafe { self.end_ptr.offset_from(self.next_ptr) };
        if offset < 0 {
            0
        } else {
            offset as usize
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::vm::types::Number;

    use super::*;

    #[test]
    fn iter_returns_all_elements() {
        let proc = ByteCodeProc {
            name: "".to_string(),
            arg_count: 0,
            bytecode: vec![
                Instruction::PushVal(Val::Void),
                Instruction::Eval(10),
                Instruction::Jump(10),
            ],
        };
        assert_eq!(
            ByteCodeIter::from_proc(proc.into()).collect::<Vec<_>>(),
            vec![
                Instruction::PushVal(Val::Void),
                Instruction::Eval(10),
                Instruction::Jump(10)
            ]
        );
    }

    #[test]
    fn jump_skips_elements() {
        let proc = ByteCodeProc {
            name: "".to_string(),
            arg_count: 0,
            bytecode: vec![
                Instruction::PushVal(Val::Number(Number::Int(0))),
                Instruction::PushVal(Val::Number(Number::Int(1))),
                Instruction::PushVal(Val::Number(Number::Int(2))),
                Instruction::PushVal(Val::Number(Number::Int(3))),
                Instruction::PushVal(Val::Number(Number::Int(4))),
            ],
        };
        let mut iter = ByteCodeIter::from_proc(proc.into());
        iter.jump(2);
        assert_eq!(
            iter.collect::<Vec<_>>(),
            vec![
                Instruction::PushVal(Val::Number(Number::Int(2))),
                Instruction::PushVal(Val::Number(Number::Int(3))),
                Instruction::PushVal(Val::Number(Number::Int(4))),
            ]
        );
    }

    #[test]
    fn count_returns_number_of_instructions() {
        let proc = ByteCodeProc {
            name: "".to_string(),
            arg_count: 0,
            bytecode: std::iter::repeat(Instruction::PushVal(Val::Void))
                .take(10)
                .collect(),
        };
        let iter = ByteCodeIter::from_proc(proc.into());
        assert_eq!(iter.count(), 10);
    }

    #[test]
    fn next_and_count_returns_remaining_number_of_instructions() {
        let proc = ByteCodeProc {
            name: "".to_string(),
            arg_count: 0,
            bytecode: std::iter::repeat(Instruction::PushVal(Val::Void))
                .take(10)
                .collect(),
        };
        let mut iter = ByteCodeIter::from_proc(proc.into());
        iter.next();
        assert_eq!(iter.count(), 9);
    }

    #[test]
    fn count_with_no_more_elements_returns_0() {
        let proc = ByteCodeProc {
            name: "".to_string(),
            arg_count: 0,
            bytecode: std::iter::repeat(Instruction::PushVal(Val::Void))
                .take(10)
                .collect(),
        };
        let mut iter = ByteCodeIter::from_proc(proc.into());
        while iter.next().is_some() {}
        assert_eq!(iter.count(), 0);
    }
}
