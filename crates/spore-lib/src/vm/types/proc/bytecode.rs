use std::rc::Rc;

use crate::vm::{module::ModuleSource, types::instruction::Instruction};

/// A procedure that can be evaluated in a vm.
#[derive(Clone)]
pub struct ByteCodeProc {
    /// The name of the procedure.
    pub name: String,
    /// The number of arguments to the procedure.
    pub arg_count: usize,
    /// The bytecode to run.
    pub bytecode: Vec<Instruction>,
    /// The module for the procedure.
    pub module: ModuleSource,
    /// True if the procedure defines a module.
    pub is_module_definition: bool,
}

impl PartialEq for ByteCodeProc {
    fn eq(&self, other: &Self) -> bool {
        self.bytecode.as_ptr() == other.bytecode.as_ptr()
    }
}

impl std::fmt::Debug for ByteCodeProc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Procedure")
            .field("name", &self.name)
            .field("module", &self.module)
            .field("is_module_definition", &self.is_module_definition)
            .finish_non_exhaustive()
    }
}

impl std::fmt::Display for ByteCodeProc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<proc {name}>", name = &self.name)
    }
}

/// An iterator over bytecode.
#[derive(Clone)]
pub struct ByteCodeIter {
    proc: Rc<ByteCodeProc>,
    next_ptr: *const Instruction,
    end_ptr: *const Instruction,
}

impl ByteCodeIter {
    /// Create a new iterator with the given `name` that returns immediately.
    pub fn new(name: String) -> ByteCodeIter {
        ByteCodeIter {
            proc: Rc::new(ByteCodeProc {
                name,
                arg_count: 0,
                bytecode: Vec::new(),
                module: ModuleSource::Global,
                is_module_definition: false,
            }),
            next_ptr: std::ptr::null(),
            end_ptr: std::ptr::null(),
        }
    }

    pub fn inner(&self) -> &Rc<ByteCodeProc> {
        &self.proc
    }

    /// Create an interator over a `ByteCodeProc` shared reference.
    pub fn from_proc(proc: Rc<ByteCodeProc>) -> ByteCodeIter {
        let next = proc.bytecode.as_ptr();
        let end = unsafe { next.add(proc.bytecode.len()) };
        ByteCodeIter {
            proc,
            next_ptr: next,
            end_ptr: end,
        }
    }
}

impl ByteCodeIter {
    /// Jump some number of instructions.
    pub fn jump(&mut self, n: usize) {
        self.next_ptr = unsafe { self.next_ptr.add(n).min(self.end_ptr) };
    }

    /// Get the next instruction or `Instruction::Return` if there are no more instructions.
    pub fn next_instruction(&mut self) -> &Instruction {
        if self.next_ptr < self.end_ptr {
            let res = unsafe { &*self.next_ptr };
            self.next_ptr = unsafe { self.next_ptr.offset(1) };
            res
        } else {
            &Instruction::Return
        }
    }
}

impl Iterator for ByteCodeIter {
    type Item = Instruction;

    fn next(&mut self) -> Option<Instruction> {
        if self.next_ptr < self.end_ptr {
            let res = unsafe { &*self.next_ptr };
            self.next_ptr = unsafe { self.next_ptr.offset(1) };
            Some(res.clone())
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::vm::types::Val;

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
            module: ModuleSource::Virtual(""),
            is_module_definition: false,
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
                Instruction::PushVal(0.into()),
                Instruction::PushVal(1.into()),
                Instruction::PushVal(2.into()),
                Instruction::PushVal(3.into()),
                Instruction::PushVal(4.into()),
            ],
            module: ModuleSource::Virtual(""),
            is_module_definition: false,
        };
        let mut iter = ByteCodeIter::from_proc(proc.into());
        iter.jump(2);
        assert_eq!(
            iter.collect::<Vec<_>>(),
            vec![
                Instruction::PushVal(2.into()),
                Instruction::PushVal(3.into()),
                Instruction::PushVal(4.into()),
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
            module: ModuleSource::Virtual(""),
            is_module_definition: false,
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
            module: ModuleSource::Virtual(""),
            is_module_definition: false,
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
            module: ModuleSource::Virtual(""),
            is_module_definition: false,
        };
        let mut iter = ByteCodeIter::from_proc(proc.into());
        while iter.next().is_some() {}
        assert_eq!(iter.count(), 0);
    }
}
