use compact_str::CompactString;

use crate::Vm;

use super::{
    bytecode::ByteCode, custom::CustomVal, formatter::ValFormatter, ListVal, NativeFunction, ValId,
};

/// Contains a Spore value.
///
/// # Safety
///The value is considered unsafe as some variants contain references that may be mutated or garbage
/// collected by the VM. Unsafe fields contain a `Safety` section in their documentation.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub enum UnsafeVal {
    /// A type that contains a single value. Used to represent nothingness.
    #[default]
    Void,
    /// Either true or false.
    Bool(bool),
    /// A 64 bit signed integer
    Int(i64),
    /// A 64 bit floating point number.
    Float(f64),
    /// A handle to a string within the VM.
    ///
    /// # Safety
    /// May be garbage collected or mutated by the VM.
    String(ValId<CompactString>),
    /// A handle to a box containing a mutable value.
    ///
    /// # Safety
    /// May be garbage collected or mutated by the VM.
    MutableBox(ValId<UnsafeVal>),
    /// A handle to a list within the VM.
    ///
    /// # Safety
    /// May be garbage collected or mutated by the VM.
    List(ValId<ListVal>),
    /// A handle to a function implemented in Spore's bytecode.
    ///
    /// # Safety
    /// May be garbage collected or mutated by the VM.
    ByteCodeFunction(ValId<ByteCode>),
    /// A function implemented in Rust.
    NativeFunction(NativeFunction),
    /// A handle to a custom type.
    ///
    /// # Safety
    /// May be garbage collected or mutated by the VM.
    Custom(ValId<CustomVal>),
}

impl UnsafeVal {
    /// The display name for the function type.
    pub const FUNCTION_TYPE_NAME: &'static str = "function";
    /// The display name for the boolean type.
    pub const BOOL_TYPE_NAME: &'static str = "bool";
    /// The display name for the integer type.
    pub const INT_TYPE_NAME: &'static str = "int";
    /// The display name for the float type.
    pub const FLOAT_TYPE_NAME: &'static str = "float";
    /// The display name for the void type.
    pub const VOID_TYPE_NAME: &'static str = "void";
    /// The display name for the string type.
    pub const STRING_TYPE_NAME: &'static str = "string";
    /// The display name for the mutable box type.
    pub const MUTABLE_BOX_TYPE_NAME: &'static str = "mutable-box";
    /// The display name for the list type.
    pub const LIST_TYPE_NAME: &'static str = "list";
    /// The display name for the custom type.
    pub const CUSTOM_TYPE_NAME: &'static str = "custom";

    /// Get the display name for the type of `self`.
    pub fn type_name(self) -> &'static str {
        match self {
            UnsafeVal::Void => UnsafeVal::VOID_TYPE_NAME,
            UnsafeVal::Bool(_) => UnsafeVal::BOOL_TYPE_NAME,
            UnsafeVal::Int(_) => UnsafeVal::INT_TYPE_NAME,
            UnsafeVal::Float(_) => UnsafeVal::FLOAT_TYPE_NAME,
            UnsafeVal::String(_) => UnsafeVal::STRING_TYPE_NAME,
            UnsafeVal::MutableBox(_) => UnsafeVal::MUTABLE_BOX_TYPE_NAME,
            UnsafeVal::List(_) => UnsafeVal::LIST_TYPE_NAME,
            UnsafeVal::ByteCodeFunction(_) => UnsafeVal::FUNCTION_TYPE_NAME,
            UnsafeVal::NativeFunction(_) => UnsafeVal::FUNCTION_TYPE_NAME,
            UnsafeVal::Custom(_) => UnsafeVal::CUSTOM_TYPE_NAME,
        }
    }

    /// Returns `false` if `self` is `void` or `false`. All other values return `true`.
    pub fn is_truthy(self) -> bool {
        !matches!(self, UnsafeVal::Void | UnsafeVal::Bool(false))
    }

    /// Get a display formatter for the current value.
    pub fn formatted<'a>(&self, vm: &'a Vm) -> impl 'a + std::fmt::Display {
        ValFormatter::new(vm, *self)
    }

    /// Get a display formatter for the current type. Unlike [Self::formatted], this will print
    /// strings quoted. For example, `"hello"` will display as "hello" instead of hello.
    pub fn format_quoted<'a>(&self, vm: &'a Vm) -> impl 'a + std::fmt::Display {
        ValFormatter::new_quoted(vm, *self)
    }
}

macro_rules! to_internal_val_impl {
    ($rust_type:ty => $variant:ident) => {
        impl From<$rust_type> for UnsafeVal {
            fn from(v: $rust_type) -> UnsafeVal {
                UnsafeVal::$variant(v)
            }
        }
    };
}

to_internal_val_impl!(bool => Bool);
to_internal_val_impl!(i64 => Int);
to_internal_val_impl!(f64 => Float);
to_internal_val_impl!(NativeFunction => NativeFunction);
to_internal_val_impl!(ValId<CompactString> => String);
to_internal_val_impl!(ValId<UnsafeVal> => MutableBox);
to_internal_val_impl!(ValId<ListVal> => List);
to_internal_val_impl!(ValId<ByteCode> => ByteCodeFunction);
to_internal_val_impl!(ValId<CustomVal> => Custom);

impl From<()> for UnsafeVal {
    fn from(_: ()) -> UnsafeVal {
        UnsafeVal::Void
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn internal_val_is_small() {
        assert_eq!(
            std::mem::size_of::<UnsafeVal>(),
            2 * std::mem::size_of::<usize>()
        );
    }

    #[test]
    fn hacks_for_code_coverage() {
        let vals = [
            UnsafeVal::Void,
            UnsafeVal::Bool(false),
            UnsafeVal::Int(0),
            UnsafeVal::Float(0.0),
            UnsafeVal::String(Default::default()),
            UnsafeVal::MutableBox(Default::default()),
            UnsafeVal::List(Default::default()),
            UnsafeVal::ByteCodeFunction(Default::default()),
            UnsafeVal::NativeFunction(crate::builtins::add),
            UnsafeVal::Custom(ValId {
                vm_id: 0,
                obj_id: 0,
                idx: 0,
                _marker: std::marker::PhantomData,
            }),
        ];
        for v in vals {
            assert_ne!(v.type_name(), "");
        }
    }
}
