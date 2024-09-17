use smol_str::SmolStr;

use super::{InternalVal, NativeFunction};

/// Contains a set of instructions for the Spore VM to evaluate.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct ByteCode {
    /// The name of the function.
    pub name: SmolStr,
    /// The number of arguments for the bytecode.
    pub arg_count: usize,
    /// The instructions for the bytecode.
    pub instructions: Box<[Instruction]>,
}

impl ByteCode {
    pub fn values(&self) -> impl '_ + Iterator<Item = InternalVal> {
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
            })
    }
}

/// An instruction for the VM to execute.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Instruction {
    /// Push a constant onto the stack.
    PushConst(InternalVal),
    /// Push the current function onto the stack.
    PushCurrentFunction,
    /// Get the nth argument from the start of the continuation's stack.
    GetArg(usize),
    /// Get the value of a symbol at push it onto the stack.
    Deref(SmolStr),
    /// Pop the top value of the stack and assign it to the given symbol.
    Define(SmolStr),
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;

    #[test]
    fn struct_sizes_are_small_enough() {
        assert_eq!(size_of::<ByteCode>(), 6 * size_of::<usize>());
        assert_eq!(size_of::<Instruction>(), 4 * size_of::<usize>());
    }
}
