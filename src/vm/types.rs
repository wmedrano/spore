use anyhow::{anyhow, Result};
use std::{borrow::Borrow, sync::Arc};

use super::environment::Environment;

/// Contains a single value.
#[derive(Clone, Debug, PartialEq)]
pub enum Val {
    Void,
    String(Arc<String>),
    Symbol(Symbol),
    Bool(bool),
    Number(Number),
    Proc(Arc<Procedure>),
    List(Box<Vec<Val>>),
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
            Val::String(x) => write!(f, "{:?}", x),
            Val::Symbol(x) => write!(f, "{}", x),
            Val::Bool(x) => write!(f, "{x}"),
            Val::Number(x) => match x {
                Number::Int(x) => write!(f, "{x}"),
                Number::Float(x) => write!(f, "{x}"),
            },
            Val::Proc(x) => write!(f, "{:}", x),
            Val::List(xs) => {
                write!(f, "(")?;
                let mut items = xs.iter();
                if let Some(item) = items.next() {
                    write!(f, "{item}")?;
                }
                for item in items {
                    write!(f, " {item}")?;
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

impl_enum_from!(Val, String => String);
impl_enum_from!(Val, Symbol => Symbol);
impl_enum_from!(Val, Number => Number);
impl_enum_from!(Val, bool => Bool);
impl_enum_from!(Val, Procedure => Proc);
impl_enum_from!(Val, Vec<Val> => List);

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

/// A symbol.
#[derive(Clone, Debug, PartialEq, Hash, Eq)]
pub struct Symbol(Arc<String>);

impl Symbol {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl<'a> From<&'a str> for Symbol {
    fn from(s: &'a str) -> Symbol {
        Symbol(Arc::new(s.to_string()))
    }
}

impl From<String> for Symbol {
    fn from(s: String) -> Symbol {
        Symbol(Arc::new(s))
    }
}

impl AsRef<str> for Symbol {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Borrow<str> for Symbol {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl std::fmt::Display for Symbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "'{}", self.as_str())
    }
}

pub trait GenericProcedure: 'static + Send + Sync {
    /// The name of the procedure.
    fn name(&self) -> &str;

    /// Evaluate the procedure on the environment.
    fn eval(&self, env: &mut Environment) -> Result<Val>;
}

impl<P> GenericProcedure for (&'static str, P)
where
    P: 'static + Send + Sync + Fn(&[Val]) -> Result<Val>,
{
    /// The name of the procedure.
    fn name(&self) -> &str {
        self.0
    }

    /// Evaluate the procedure in the given environment.
    fn eval(&self, env: &mut Environment) -> Result<Val> {
        let start = env.local.stack_base;
        let end = env.stack.len();
        (self.1)(&env.stack[start..end])
    }
}

/// A function.
pub struct Procedure {
    inner: Box<dyn GenericProcedure>,
}

impl Procedure {
    /// Create a new function.
    pub fn with_native<P: 'static + Send + Sync + Fn(&[Val]) -> Result<Val>>(
        name: &'static str,
        proc: P,
    ) -> Arc<Procedure> {
        let f = Box::new((name, proc));
        Arc::new(Procedure { inner: f })
    }

    /// Create a new function.
    pub fn new<P: GenericProcedure>(proc: P) -> Arc<Procedure> {
        let f: Box<dyn GenericProcedure> = Box::new(proc);
        Arc::new(Procedure { inner: f })
    }

    pub fn name(&self) -> &str {
        self.inner.name()
    }
}

impl Procedure {
    /// Evaluate the function with the given arguments.
    pub fn eval(&self, env: &mut Environment) -> Result<Val> {
        self.inner.as_ref().eval(env)
    }
}

impl PartialEq for Procedure {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.inner.as_ref(), other.inner.as_ref())
    }
}

impl std::fmt::Debug for Procedure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = self.name();
        f.debug_struct("Function")
            .field("name", &name)
            .finish_non_exhaustive()
    }
}

impl std::fmt::Display for Procedure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<proc {name}>", name = self.name())
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
