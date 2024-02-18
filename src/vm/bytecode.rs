use super::types::Val;

#[derive(Clone, Debug)]
pub enum OpCode {
    /// Push a new value to the stack.
    PushVal(Val),
    /// Evaluate the top of the stack.
    Eval(usize),
    /// Jump a number of bytecodes if the result at the top of the stack is false.
    JumpIf(usize),
    /// Jump ahead by the given number of bytecode instructions.
    Jump(usize),
}

impl std::fmt::Display for OpCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OpCode::PushVal(v) => write!(f, "push value {v}"),
            OpCode::Eval(n) => write!(f, "evaluate last {n}"),
            OpCode::JumpIf(n) => write!(f, "jump {n} if true"),
            OpCode::Jump(n) => write!(f, "jump {n}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn byte_code_size_is_small() {
        assert_eq!(
            std::mem::size_of::<OpCode>(),
            2 * std::mem::size_of::<usize>()
        )
    }
}
