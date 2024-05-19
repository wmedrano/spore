use anyhow::{anyhow, bail, Result};
use std::rc::Rc;

use self::{
    proc::{bytecode::ByteCodeProc, native::NativeProc},
    symbol::Symbol,
};

pub mod instruction;
pub mod proc;
pub mod symbol;

/// Contains a single value.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Val {
    #[default]
    Void,
    Bool(bool),
    Int(isize),
    Float(f64),
    ByteCodeProc(Rc<ByteCodeProc>),
    NativeProc(Rc<NativeProc>),
    String(Rc<String>),
    Symbol(Symbol),
    List(Rc<Vec<Val>>),
}

impl Val {
    /// Returns true if `self` is a `true`. If `self` is not a bool, then an error is returned.
    pub fn is_truthy(&self) -> Result<bool> {
        match self {
            Val::Bool(v) => Ok(*v),
            v => Err(anyhow!("expected true/false, but found {}", v)),
        }
    }

    /// Returns a static string representing the type name of `self`.
    ///
    /// This method is useful for debugging and type checking. It provides a human-readable label
    /// for the type of the value without the need to match on the variant itself.
    pub fn type_name(&self) -> &'static str {
        match self {
            Val::Void => "<void>",
            Val::Bool(_) => "<bool>",
            Val::Int(_) => "<int>",
            Val::Float(_) => "<float>",
            Val::ByteCodeProc(_) | Val::NativeProc(_) => "<proc>",
            Val::String(_) => "<string>",
            Val::Symbol(_) => "<symbol>",
            Val::List(_) => "<list>",
        }
    }

    /// Attempts to convert `self` into an `isize`.
    ///
    /// If `self` is not an `Val::Int`, this method returns an error.
    ///
    /// # Errors
    ///
    /// Returns an error if `self` is not of type `<int>`.
    pub fn try_int(&self) -> Result<isize> {
        match self {
            Val::Int(v) => Ok(*v),
            _ => bail!("expected <int> but found {}", self.type_name()),
        }
    }

    /// Attempts to convert `self` into a `&str`.
    ///
    /// If `self` is not an `Val::String`, this method returns an error.
    ///
    /// # Errors
    ///
    /// Returns an error if `self` is not of type `<string>`.
    pub fn try_str(&self) -> Result<&str> {
        match self {
            Val::String(v) => Ok(v.as_str()),
            _ => bail!("expected <string> but found {}", self.type_name()),
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
            Val::Bool(x) => write!(f, "{x}"),
            Val::Int(x) => write!(f, "{x}"),
            Val::Float(x) => write!(f, "{x}"),
            Val::ByteCodeProc(x) => write!(f, "{}", x),
            Val::NativeProc(x) => write!(f, "{}", x),
            Val::String(x) => write!(f, "{:?}", x),
            Val::Symbol(x) => write!(f, "{}", x),
            Val::List(x) => {
                write!(f, "(")?;
                for (idx, v) in x.iter().enumerate() {
                    if idx > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{v}")?;
                }
                write!(f, ")")
            }
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

impl_enum_from!(Val, bool => Bool);
impl_enum_from!(Val, isize => Int);
impl_enum_from!(Val, f64 => Float);
impl_enum_from!(Val, ByteCodeProc => ByteCodeProc);
impl_enum_from!(Val, NativeProc => NativeProc);
impl_enum_from!(Val, Symbol => Symbol);
impl_enum_from!(Val, String => String);

impl From<&str> for Val {
    fn from(s: &str) -> Val {
        Val::from(s.to_string())
    }
}

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
        assert_ne!(Val::from(100.0), Val::from(100));
    }

    #[test]
    fn numbers_are_equal() {
        assert_eq!(Val::from(100), Val::from(100));
        assert_eq!(Val::from(100.0), Val::from(100.0));
    }

    #[test]
    fn string_equality() {
        assert_eq!(Val::from("value"), Val::from("value"));
        assert_ne!(Val::from("value1"), Val::from("value2"));
        assert_eq!(Val::from(""), Val::from(""));
    }
}
