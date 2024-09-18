use std::sync::Arc;

use smol_str::SmolStr;

use crate::{val_store::ValId, Vm};

use super::{bytecode::ByteCode, formatter::ValFormatter, NativeFunction};

pub type ListVal = Vec<InternalVal>;

/// Contains a Spore value.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub enum InternalVal {
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
    String(ValId<SmolStr>),
    /// A list.
    List(ValId<ListVal>),
    /// A function implemented in Spore's bytecode.
    ByteCodeFunction(ValId<Arc<ByteCode>>),
    /// A function implemented in Rust.
    NativeFunction(NativeFunction),
}

impl InternalVal {
    pub const FUNCTION_TYPE_NAME: &'static str = "function";
    pub const BOOL_TYPE_NAME: &'static str = "bool";
    pub const INT_TYPE_NAME: &'static str = "int";
    pub const FLOAT_TYPE_NAME: &'static str = "float";
    pub const VOID_TYPE_NAME: &'static str = "void";
    pub const STRING_TYPE_NAME: &'static str = "string";
    pub const LIST_TYPE_NAME: &'static str = "list";

    pub fn type_name(&self) -> &'static str {
        match self {
            InternalVal::Void => InternalVal::VOID_TYPE_NAME,
            InternalVal::Bool(_) => InternalVal::BOOL_TYPE_NAME,
            InternalVal::Int(_) => InternalVal::INT_TYPE_NAME,
            InternalVal::Float(_) => InternalVal::FLOAT_TYPE_NAME,
            InternalVal::String(_) => InternalVal::STRING_TYPE_NAME,
            InternalVal::List(_) => InternalVal::LIST_TYPE_NAME,
            InternalVal::ByteCodeFunction(_) => InternalVal::FUNCTION_TYPE_NAME,
            InternalVal::NativeFunction(_) => InternalVal::FUNCTION_TYPE_NAME,
        }
    }

    pub fn formatted<'a>(&self, vm: &'a Vm) -> impl 'a + std::fmt::Display {
        ValFormatter::new(vm, *self)
    }

    pub fn format_quoted<'a>(&self, vm: &'a Vm) -> impl 'a + std::fmt::Display {
        ValFormatter::new_quoted(vm, *self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn internal_val_is_small() {
        assert_eq!(
            std::mem::size_of::<InternalVal>(),
            2 * std::mem::size_of::<usize>()
        );
    }
}
