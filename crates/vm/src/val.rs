use crate::error::VmResult;

/// Contains a Spore value.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) enum InternalVal {
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
    NativeFunction(fn(&crate::Vm, &[InternalVal]) -> VmResult<InternalVal>),
}

impl InternalVal {
    pub const FUNCTION_TYPE_NAME: &'static str = "function";
    pub const BOOL_TYPE_NAME: &'static str = "bool";
    pub const INT_TYPE_NAME: &'static str = "int";
    pub const FLOAT_TYPE_NAME: &'static str = "float";
    pub const VOID_TYPE_NAME: &'static str = "void";
    pub const STRING_TYPE_NAME: &'static str = "string";

    pub fn type_name(&self) -> &'static str {
        match self {
            InternalVal::Void => InternalVal::VOID_TYPE_NAME,
            InternalVal::Bool(_) => InternalVal::BOOL_TYPE_NAME,
            InternalVal::Int(_) => InternalVal::INT_TYPE_NAME,
            InternalVal::Float(_) => InternalVal::FLOAT_TYPE_NAME,
            InternalVal::String(_) => InternalVal::STRING_TYPE_NAME,
            InternalVal::ByteCodeFunction(_) => InternalVal::FUNCTION_TYPE_NAME,
            InternalVal::NativeFunction(_) => InternalVal::FUNCTION_TYPE_NAME,
        }
    }
}

/// Contains a set of instructions for the Spore VM to evaluate.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct ByteCode {
    /// The name of the function.
    pub name: String,
    /// The number of arguments for the bytecode.
    pub arg_count: usize,
    /// The instructions for the bytecode.
    pub instructions: Vec<Instruction>,
}

/// An instruction for the VM to execute.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Instruction {
    /// Push a constant onto the stack.
    PushConst(InternalVal),
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

#[derive(Clone, PartialEq, Debug)]
pub struct Val {
    pub(crate) v: InternalVal,
}

impl Val {
    pub fn is_void(&self) -> bool {
        matches!(self.v, InternalVal::Void)
    }
}

impl std::fmt::Display for Val {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.v {
            InternalVal::Void => Ok(()),
            InternalVal::Bool(x) => write!(f, "{x}"),
            InternalVal::Int(x) => write!(f, "{x}"),
            InternalVal::Float(x) => write!(f, "{x}"),
            InternalVal::String(x) => write!(f, "{x}"),
            InternalVal::ByteCodeFunction(bc) => write!(
                f,
                "<function {name}>",
                name = if bc.name.is_empty() {
                    "_"
                } else {
                    bc.name.as_str()
                }
            ),
            InternalVal::NativeFunction(_) => write!(f, "<native-function>"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Vm;

    #[test]
    fn format_void_is_empty() {
        assert_eq!(
            Val {
                v: InternalVal::Void
            }
            .to_string(),
            ""
        );
    }

    #[test]
    fn format_bool_is_true_or_false() {
        assert_eq!(
            Val {
                v: InternalVal::Bool(true)
            }
            .to_string(),
            "true"
        );
        assert_eq!(
            Val {
                v: InternalVal::Bool(false)
            }
            .to_string(),
            "false"
        );
    }

    #[test]
    fn format_int_prints_number() {
        assert_eq!(
            Val {
                v: InternalVal::Int(0)
            }
            .to_string(),
            "0"
        );
        assert_eq!(
            Val {
                v: InternalVal::Int(-1)
            }
            .to_string(),
            "-1"
        );
    }

    #[test]
    fn format_float_prints_number() {
        assert_eq!(
            Val {
                v: InternalVal::Float(0.0)
            }
            .to_string(),
            "0"
        );
        assert_eq!(
            Val {
                v: InternalVal::Float(-1.5)
            }
            .to_string(),
            "-1.5"
        );
    }

    #[test]
    fn format_string_produces_string() {
        assert_eq!(
            Val {
                v: InternalVal::String("my string".to_string())
            }
            .to_string(),
            "my string"
        );
    }

    #[test]
    fn format_function_prints_name() {
        let mut vm = Vm::new();
        let v = vm.eval_str("(define (foo) 10) foo").unwrap();
        assert_eq!(v.to_string(), "<function foo>",);
    }

    #[test]
    fn format_native_function_prints_native_function() {
        let mut vm = Vm::new();
        let v = vm.eval_str("+").unwrap();
        assert_eq!(v.to_string(), "<native-function>");
    }
}
