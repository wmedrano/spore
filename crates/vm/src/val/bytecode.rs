use std::sync::Arc;

use compact_str::CompactString;

use crate::parser::tokenizer::Span;

use super::{NativeFunction, UnsafeVal};

/// Contains a set of instructions for the Spore VM to evaluate.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ByteCode {
    /// The name of the function.
    pub name: CompactString,
    /// The number of arguments for the bytecode.
    pub arg_count: usize,
    /// The instructions for the bytecode.
    pub instructions: Arc<[Instruction]>,
    /// The source code for the bytecode.
    pub source: Option<Arc<str>>,
    /// The span containing the instruction code from `source`.
    pub instruction_source: Box<[Span]>,
}

impl ByteCode {
    /// Create bytecode that calls `function` with the top `arg_count` args in the stack.
    pub fn new_native_function_call(
        name: &str,
        func: NativeFunction,
        arg_count: usize,
    ) -> ByteCode {
        ByteCode {
            name: name.into(),
            arg_count: 0,
            instructions: Arc::new([Instruction::EvalNative { func, arg_count }]),
            source: None,
            instruction_source: Box::default(),
        }
    }

    /// Iterate over all values referenced by the bytecode.
    pub fn values(&self) -> impl '_ + Iterator<Item = UnsafeVal> {
        self.instructions
            .iter()
            .flat_map(|instruction| match instruction {
                Instruction::PushConst(v) => Some(*v),
                Instruction::PushCurrentFunction => None,
                Instruction::GetArg(_) => None,
                Instruction::Deref(_) => None,
                Instruction::Define(_) => None,
                Instruction::Eval(_) => None,
                Instruction::EvalNative { .. } => None,
                Instruction::JumpIf(_) => None,
                Instruction::Jump(_) => None,
                Instruction::Return => None,
            })
    }
}

/// An instruction for the VM to execute.
#[derive(Clone, Debug, PartialEq)]
pub enum Instruction {
    /// Push a constant onto the stack.
    PushConst(UnsafeVal),
    /// Push the current function onto the stack.
    PushCurrentFunction,
    /// Get the nth argument from the start of the continuation's stack.
    GetArg(usize),
    /// Get the value of a symbol at push it onto the stack.
    Deref(CompactString),
    /// Pop the top value of the stack and assign it to the given symbol.
    Define(CompactString),
    /// Pop the top `n` values of the stack. The deepmost value should be function with the rest of
    /// the values acting as the arguments.
    Eval(usize),
    /// Pop the top `n` values of the stack. The deepmost value should be function with the rest of
    /// the values acting as the arguments.
    EvalNative {
        func: NativeFunction,
        arg_count: usize,
    },
    /// Pop the top value of the stack. If it is `true`, then jump `n` instructions.
    JumpIf(usize),
    /// Jump `n` instructions.
    Jump(usize),
    /// Return from the current function.
    Return,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;

    #[test]
    fn struct_sizes_are_small_enough() {
        assert_eq!(size_of::<Instruction>(), 4 * size_of::<usize>());
    }
}
