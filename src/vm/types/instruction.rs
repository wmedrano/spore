use super::{symbol::Symbol, Val};

#[derive(Clone, Debug, PartialEq)]
pub enum Instruction {
    /// Push a new value to the stack.
    PushVal(Val),
    /// Evaluate the top of the stack.
    Eval(usize),
    /// Jump a number of bytecodes if the result at the top of the stack is false.
    JumpIf(usize),
    /// Jump ahead by the given number of bytecode instructions.
    Jump(usize),
    /// Get the value of the given symbol from the environment and push it on the stack.
    GetSym(Symbol),
    /// Get the argument by index. The argument is determined by counting from the stack base.
    GetArg(usize),
}

impl Instruction {
    pub fn map_push_val(self, f: impl Fn(Val) -> Val) -> Instruction {
        match self {
            Instruction::PushVal(v) => Instruction::PushVal(f(v)),
            i => i,
        }
    }
}

impl std::fmt::Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Instruction::PushVal(v) => write!(f, "push value {v}"),
            Instruction::Eval(n) => write!(f, "evaluate last {n}"),
            Instruction::JumpIf(n) => write!(f, "jump {n} if true"),
            Instruction::Jump(n) => write!(f, "jump {n}"),
            Instruction::GetSym(s) => write!(f, "get symbol {s}"),
            Instruction::GetArg(n) => write!(f, "get arg {n}"),
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
