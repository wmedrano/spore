use crate::error::VmResult;

/// Contains a Spore value.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Val {
    /// A type that contains a single value. Used to represent nothingness.
    #[default]
    Void,
    /// Either true or false.
    Bool(bool),
    /// A 64 bit signed integer
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

pub struct FormattedVal<'a> {
    pub(crate) v: &'a Val,
}

impl<'a> std::fmt::Display for FormattedVal<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.v {
            Val::Void => Ok(()),
            Val::Bool(x) => write!(f, "{x}"),
            Val::Int(x) => write!(f, "{x}"),
            Val::Float(x) => write!(f, "{x}"),
            Val::String(x) => write!(f, "{x}"),
            Val::ByteCodeFunction(bc) => write!(
                f,
                "<function {name}>",
                name = if bc.name.is_empty() {
                    "_"
                } else {
                    bc.name.as_str()
                }
            ),
            Val::NativeFunction(_) => write!(f, "<native-function>"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Vm;

    #[test]
    fn format_void_is_empty() {
        let vm = Vm::new();
        assert_eq!(vm.formatted_val(&Val::Void).to_string(), "");
    }

    #[test]
    fn format_bool_is_true_or_false() {
        let vm = Vm::new();
        assert_eq!(vm.formatted_val(&Val::Bool(true)).to_string(), "true");
        assert_eq!(vm.formatted_val(&Val::Bool(false)).to_string(), "false");
    }

    #[test]
    fn format_int_prints_number() {
        let vm = Vm::new();
        assert_eq!(vm.formatted_val(&Val::Int(0)).to_string(), "0");
        assert_eq!(vm.formatted_val(&Val::Int(-1)).to_string(), "-1");
    }

    #[test]
    fn format_float_prints_number() {
        let vm = Vm::new();
        assert_eq!(vm.formatted_val(&Val::Float(0.0)).to_string(), "0");
        assert_eq!(vm.formatted_val(&Val::Float(-1.5)).to_string(), "-1.5");
    }

    #[test]
    fn format_string_produces_string() {
        let vm = Vm::new();
        assert_eq!(
            vm.formatted_val(&Val::String("my string".to_string()))
                .to_string(),
            "my string"
        );
    }

    #[test]
    fn format_function_prints_name() {
        let mut vm = Vm::new();
        let v = vm.eval_str("(define (foo) 10) foo").unwrap();
        assert_eq!(vm.formatted_val(&v).to_string(), "<function foo>",);
    }

    #[test]
    fn format_native_function_prints_native_function() {
        let mut vm = Vm::new();
        let v = vm.eval_str("+").unwrap();
        assert_eq!(vm.formatted_val(&v).to_string(), "<native-function>");
    }
}
