use std::path::PathBuf;

use smol_str::SmolStr;

use crate::vm::module::ModuleSource;

use super::{symbol::Symbol, Val};

#[derive(Clone, Debug, PartialEq)]
pub enum Instruction {
    /// Push a new value to the stack.
    PushVal(Val),
    /// Evaluate the top of the stack. The `usize` is the number of arguments + 1 for the
    /// procedure. If `usize` is 0, then it is equivalent to Eval(stack_frame_size).
    Eval(usize),
    /// Get the argument by index. The argument is determined by counting from the stack base.
    GetArg(usize),
    /// Get the value of the given symbol from the environment and push it on the stack.
    GetVal(Box<ValRef>),
    /// Jump a number of bytecodes if the result at the top of the stack is false.
    JumpIf(usize),
    /// Jump ahead by the given number of bytecode instructions.
    Jump(usize),
    /// Set the value of Symbol to be what is at the top of the stack.
    SetVal(Box<Symbol>),
    /// Load a module.
    ImportModule(Box<PathBuf>),
    /// Takes the list at the top of the stack, pops it, and pushes all its values onto the stack.
    UnwrapList,
    /// Pops the current frame and returns the value at the top of the current frame stack.
    Return,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ValRef {
    /// The module the value is referenced from.
    pub module: ModuleSource,
    /// The sub module that the value should be taken from.
    pub sub_module: Option<SmolStr>,
    /// The symbol for the value.
    pub symbol: SmolStr,
}

impl std::fmt::Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Instruction::PushVal(v) => write!(f, "push value {v}"),
            Instruction::Eval(n) => write!(f, "evaluate last {n}"),
            Instruction::JumpIf(n) => write!(f, "jump {n} if true"),
            Instruction::Jump(n) => write!(f, "jump {n}"),
            Instruction::GetArg(n) => write!(f, "get arg {n}"),
            Instruction::GetVal(s) => write!(f, "get value for {s}"),
            Instruction::SetVal(s) => write!(f, "set value for {s}"),
            Instruction::ImportModule(filepath) => write!(f, "load module {filepath:?}"),
            Instruction::UnwrapList => write!(f, "unwrap list"),
            Instruction::Return => write!(f, "return"),
        }
    }
}

impl std::fmt::Display for ValRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.sub_module {
            Some(sub) => write!(f, "{mod}/{sub}/{sym}", mod=self.module, sym=self.symbol),
            None => write!(f, "{mod}/{sym}", mod=self.module, sym = self.symbol),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instruction_size_is_small() {
        assert_eq!(
            std::mem::size_of::<Instruction>(),
            2 * std::mem::size_of::<usize>()
        )
    }
}
