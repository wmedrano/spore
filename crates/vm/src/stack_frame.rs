use std::sync::Arc;

use crate::{
    parser::span::SpanWithSource,
    val::{ByteCode, Instruction, ValId},
    Vm,
};

#[derive(Debug)]
pub struct StackFrameManager {
    pub current: StackFrame,
    pub previous: Vec<StackFrame>,
}

impl StackFrameManager {
    pub fn reset(&mut self) {
        self.reset_with_stack_frame(StackFrame::default());
    }

    pub fn reset_with_stack_frame(&mut self, stack_frame: StackFrame) {
        self.current = stack_frame;
        self.previous.clear();
    }

    pub fn at_capacity(&self) -> bool {
        self.previous.len() == self.previous.capacity()
    }

    pub fn push(&mut self, stack_frame: StackFrame) {
        self.previous
            .push(std::mem::replace(&mut self.current, stack_frame));
    }

    pub fn pop(&mut self) {
        self.current = self.previous.pop().unwrap();
    }

    pub fn iter(&self) -> impl '_ + Iterator<Item = &StackFrame> {
        self.previous.iter().chain(std::iter::once(&self.current))
    }

    pub fn stack_trace_depth(&self) -> usize {
        self.previous.len() + 1
    }
}

impl Default for StackFrameManager {
    fn default() -> StackFrameManager {
        StackFrameManager {
            current: StackFrame::default(),
            // Allocate for a function call depth of 64. This is more than enough for most programs.
            previous: Vec::with_capacity(64),
        }
    }
}

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

    /// Returns `true` if `self` holds a valid function call.
    pub fn has_valid_function_call(&self) -> bool {
        self.bytecode_id.vm_id != 0
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
