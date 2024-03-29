use super::{symbol::Symbol, Val};

#[derive(Clone, Debug, PartialEq)]
pub enum Instruction {
    /// Push a new value to the stack.
    PushVal(Val),
    /// Evaluate the top of the stack.
    Eval(usize),
    /// Get the argument by index. The argument is determined by counting from the stack base.
    GetArg(usize),
    /// Get the value of the given symbol from the environment and push it on the stack.
    GetVal(Symbol),
    /// Jump a number of bytecodes if the result at the top of the stack is false.
    JumpIf(usize),
    /// Jump ahead by the given number of bytecode instructions.
    Jump(usize),
    /// Pops the current frame and returns the value at the top of the current frame stack.
    Return,
    /// Set the given symbol to the top value of the stack.
    SetVal(Symbol),
}

impl std::fmt::Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Instruction::PushVal(v) => write!(f, "push value {v}"),
            Instruction::Eval(n) => write!(f, "evaluate last {n}"),
            Instruction::JumpIf(n) => write!(f, "jump {n} if true"),
            Instruction::Jump(n) => write!(f, "jump {n}"),
            Instruction::GetVal(s) => write!(f, "get value for {s}"),
            Instruction::SetVal(s) => write!(f, "set value for {s}"),
            Instruction::GetArg(n) => write!(f, "get arg {n}"),
            Instruction::Return => write!(f, "return"),
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
