use std::sync::Arc;

use crate::{
    parser::span::SpanWithSource,
    val::{ByteCode, Instruction, ValId},
    Vm,
};

/// Used to decide the next instruction to take.
#[derive(Clone, Debug)]
pub struct StackFrame {
    pub(crate) bytecode_id: ValId<ByteCode>,
    /// The instructions that will be taken.
    pub(crate) instructions: Arc<[Instruction]>,
    /// The index of the next instruction within bytecode.
    pub(crate) instruction_idx: usize,
    /// The index of the stack for the first value of this stack frame's local stack.
    pub(crate) stack_start: usize,
}

impl StackFrame {
    /// Create a new stack frame.
    pub fn new(
        bytecode_id: ValId<ByteCode>,
        bytecode: &ByteCode,
        stack_start: usize,
    ) -> StackFrame {
        StackFrame {
            bytecode_id,
            instructions: bytecode.instructions.clone(),
            instruction_idx: 0,
            stack_start,
        }
    }

    /// Get the underlying bytecode object.
    pub fn bytecode<'a>(&self, vm: &'a Vm) -> &'a ByteCode {
        vm.objects.get_bytecode(self.bytecode_id).unwrap()
    }

    /// Get the source for the last instruction.
    pub fn previous_instruction_source(&self, vm: &Vm) -> Option<SpanWithSource<Arc<str>>> {
        let instruction_idx = self.instruction_idx.saturating_sub(1);
        let bytecode = vm.objects.get_bytecode(self.bytecode_id)?;
        let src = bytecode.source.as_ref()?;
        let span = bytecode.instruction_source.get(instruction_idx)?;
        Some(span.with_src(src.clone()))
    }
}

impl Default for StackFrame {
    fn default() -> StackFrame {
        StackFrame {
            bytecode_id: ValId {
                vm_id: 0,
                obj_id: 0,
                idx: 0,
                _marker: std::marker::PhantomData,
            },
            instructions: Arc::default(),
            instruction_idx: 0,
            stack_start: 0,
        }
    }
}
