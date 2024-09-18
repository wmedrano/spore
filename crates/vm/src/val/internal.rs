use std::sync::Arc;

use smol_str::SmolStr;

use crate::Vm;

use super::{
    bytecode::ByteCode, custom::CustomVal, formatter::ValFormatter, NativeFunction, ValId,
};

pub type ListVal = Vec<InternalVal>;

/// Contains a Spore value.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub(crate) enum InternalValImpl {
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
    /// A custom type.
    Custom(ValId<CustomVal>),
}

/// Contains a Spore value.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct InternalVal(pub(crate) InternalValImpl);

impl InternalVal {
    pub const FUNCTION_TYPE_NAME: &'static str = "function";
    pub const BOOL_TYPE_NAME: &'static str = "bool";
    pub const INT_TYPE_NAME: &'static str = "int";
    pub const FLOAT_TYPE_NAME: &'static str = "float";
    pub const VOID_TYPE_NAME: &'static str = "void";
    pub const STRING_TYPE_NAME: &'static str = "string";
    pub const LIST_TYPE_NAME: &'static str = "list";
    pub const CUSTOM_TYPE_NAME: &'static str = "custom";

    pub fn type_name(&self) -> &'static str {
        match self.0 {
            InternalValImpl::Void => InternalVal::VOID_TYPE_NAME,
            InternalValImpl::Bool(_) => InternalVal::BOOL_TYPE_NAME,
            InternalValImpl::Int(_) => InternalVal::INT_TYPE_NAME,
            InternalValImpl::Float(_) => InternalVal::FLOAT_TYPE_NAME,
            InternalValImpl::String(_) => InternalVal::STRING_TYPE_NAME,
            InternalValImpl::List(_) => InternalVal::LIST_TYPE_NAME,
            InternalValImpl::ByteCodeFunction(_) => InternalVal::FUNCTION_TYPE_NAME,
            InternalValImpl::NativeFunction(_) => InternalVal::FUNCTION_TYPE_NAME,
            InternalValImpl::Custom(_) => InternalVal::CUSTOM_TYPE_NAME,
        }
    }

    pub fn formatted<'a>(&self, vm: &'a Vm) -> impl 'a + std::fmt::Display {
        ValFormatter::new(vm, *self)
    }

    pub fn format_quoted<'a>(&self, vm: &'a Vm) -> impl 'a + std::fmt::Display {
        ValFormatter::new_quoted(vm, *self)
    }
}

macro_rules! to_internal_val_impl {
    ($rust_type:ty => $variant:ident) => {
        impl From<$rust_type> for InternalVal {
            fn from(v: $rust_type) -> InternalVal {
                InternalVal(InternalValImpl::$variant(v))
            }
        }
    };
}

to_internal_val_impl!(bool => Bool);
to_internal_val_impl!(i64 => Int);
to_internal_val_impl!(f64 => Float);
to_internal_val_impl!(NativeFunction => NativeFunction);
to_internal_val_impl!(ValId<SmolStr> => String);
to_internal_val_impl!(ValId<ListVal> => List);
to_internal_val_impl!(ValId<Arc<ByteCode>> => ByteCodeFunction);
to_internal_val_impl!(ValId<CustomVal> => Custom);

impl From<()> for InternalVal {
    fn from(_: ()) -> InternalVal {
        InternalVal(InternalValImpl::Void)
    }
}

impl From<InternalValImpl> for InternalVal {
    fn from(v: InternalValImpl) -> InternalVal {
        InternalVal(v)
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
