use crate::error::VmResult;

/// Contains a Spore value.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Val {
    /// A type that contains a single value. Used to represent nothingness.
    #[default]
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
    ByteCodeFunction(ByteCode),
    /// A function implemented in Rust.
    NativeFunction(fn(&crate::Vm, &[Val]) -> VmResult<Val>),
}

impl Val {
    pub const FUNCTION_TYPE_NAME: &'static str = "function";
    pub const BOOL_TYPE_NAME: &'static str = "bool";
    pub const INT_TYPE_NAME: &'static str = "int";
    pub const FLOAT_TYPE_NAME: &'static str = "float";
    pub const VOID_TYPE_NAME: &'static str = "void";
    pub const STRING_TYPE_NAME: &'static str = "string";

    pub fn type_name(&self) -> &'static str {
        match self {
            Val::Void => Val::VOID_TYPE_NAME,
            Val::Bool(_) => Val::BOOL_TYPE_NAME,
            Val::Int(_) => Val::INT_TYPE_NAME,
            Val::Float(_) => Val::FLOAT_TYPE_NAME,
            Val::String(_) => Val::STRING_TYPE_NAME,
            Val::ByteCodeFunction(_) => Val::FUNCTION_TYPE_NAME,
            Val::NativeFunction(_) => Val::FUNCTION_TYPE_NAME,
        }
    }

    pub fn as_byte_code_function(&self) -> Option<&ByteCode> {
        if let Self::ByteCodeFunction(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

/// Contains a set of instructions for the Spore VM to evaluate.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ByteCode {
    /// The name of the function.
    pub name: String,
    /// The number of arguments for the bytecode.
    pub arg_count: usize,
    /// The instructions for the bytecode.
    pub instructions: Vec<Instruction>,
}

/// An instruction for the VM to execute.
#[derive(Clone, Debug, PartialEq)]
pub enum Instruction {
    /// Push a constant onto the stack.
    PushConst(Val),
    /// Get the nth argument from the start of the continuation's stack.
    GetArg(usize),
    /// Get the value of a symbol at push it onto the stack.
    Deref(String),
    /// Pop the top value of the stack and assign it to the given symbol.
    Define(String),
    /// Pop the top `n` values of the stack. The deepmost value should be function with the rest of
    /// the values acting as the arguments.
    Eval(usize),
    /// Pop the top value of the stack. If it is `true`, then jump `n` instructions.
    JumpIf(usize),
    /// Jump `n` instructions.
    Jump(usize),
}
