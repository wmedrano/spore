use crate::error::VmResult;

/// Contains a Spore value.
#[derive(Clone, Debug, PartialEq)]
pub enum Val {
    /// A type that continues a single value. Used to represent nothingness.
    Void,
    /// Either true or false.
    Bool(bool),
    /// A 64 bit signed integer.
    Int(i64),
    /// A 64 bit floating point number.
    Float(f64),
    /// A string.
    String(String),
    /// A function implemented in Spore's bytecode.
    BytecodeFunction(ByteCode),
    /// A function implemented in Rust.
    NativeFunction(fn(&[Val]) -> VmResult<Val>),
}

/// Contains a set of instructions for the Spore VM to evaluate.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ByteCode {
    /// The instructions for the bytecode.
    pub instructions: Vec<Instruction>,
}

/// An instruction for the VM to execute.
#[derive(Clone, Debug, PartialEq)]
pub enum Instruction {
    /// Push a constant onto the stack.
    PushConst(Val),
    /// Get the value of a symbol at push it onto the stack.
    Deref(String),
    /// Pop the top value of the stack and assign it to the given symbol.
    Define(String),
    /// Pop the top `n` values of the stack. The deepmost value should be function with the rest of
    /// the values acting as the arguments.
    Eval(usize),
}
