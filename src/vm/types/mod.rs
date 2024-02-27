use anyhow::{anyhow, Result};
use std::rc::Rc;

use self::symbol::Symbol;

pub mod instruction;
pub mod proc;
pub mod symbol;

/// Contains a single value.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Val {
    #[default]
    Void,
    Symbol(Symbol),
    Bool(bool),
    Number(Number),
    ByteCodeProc(Rc<proc::ByteCodeProc>),
    NativeProc(Rc<proc::NativeProc>),
}

impl Val {
    /// Returns true if `self` is a `true`. If `self` is not a bool, then an error is returned.
    pub fn is_truthy(&self) -> Result<bool> {
        match self {
            Val::Bool(v) => Ok(*v),
            v => Err(anyhow!("expected true/false, but found {}", v)),
        }
    }
}

impl AsRef<Val> for Val {
    fn as_ref(&self) -> &Val {
        self
    }
}

impl std::fmt::Display for Val {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Val::Void => write!(f, "<void>"),
            Val::Symbol(x) => write!(f, "{}", x),
            Val::Bool(x) => write!(f, "{x}"),
            Val::Number(x) => match x {
                Number::Int(x) => write!(f, "{x}"),
                Number::Float(x) => write!(f, "{x}"),
            },
            Val::ByteCodeProc(x) => write!(f, "{}", x),
            Val::NativeProc(x) => write!(f, "{}", x),
        }
    }
}

macro_rules! impl_enum_from {
    ($enum:ty, $type:ty => $variant:ident) => {
        impl From<$type> for $enum {
            fn from(value: $type) -> $enum {
                <$enum>::$variant(value.into())
            }
        }
    };
}

impl_enum_from!(Val, Symbol => Symbol);
impl_enum_from!(Val, Number => Number);
impl_enum_from!(Val, bool => Bool);
impl_enum_from!(Val, proc::ByteCodeProc => ByteCodeProc);
impl_enum_from!(Val, proc::NativeProc => NativeProc);

/// A number value.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Number {
    /// Holds an integer.
    Int(isize),
    /// Holds a floating point number.
    Float(f64),
}

impl_enum_from!(Number, isize => Int);
impl_enum_from!(Number, f64 => Float);

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::{assert_eq, assert_ne};

    #[test]
    fn val_size_is_2_words() {
        assert_eq!(std::mem::size_of::<Val>(), 2 * std::mem::size_of::<usize>());
    }

    #[test]
    fn int_and_float_are_not_equal() {
        assert_ne!(
            Val::Number(Number::Float(100.0)),
            Val::Number(Number::Int(100))
        );
    }

    #[test]
    fn ints_are_equal() {
        assert_eq!(Val::Number(Number::Int(100)), Val::Number(Number::Int(100)));
    }
}
