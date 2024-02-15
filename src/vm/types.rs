use anyhow::Result;
use std::{borrow::Borrow, sync::Arc};

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
            Val::Symbol(x) => write!(f, "'{}", x.0),
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

impl<'a> From<&'a str> for Symbol {
    fn from(s: &'a str) -> Symbol {
        Symbol(Arc::new(s.to_string()))
    }
}

impl Symbol {
    /// Get the symbol as a string.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl AsRef<str> for Symbol {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Borrow<str> for Symbol {
    fn borrow(&self) -> &str {
        self.as_ref()
    }
}

type GenericFunction = dyn 'static + Send + Sync + Fn(&[Val]) -> Result<Val>;

/// A function.
pub struct Procedure {
    name: Option<Symbol>,
    f: Box<GenericFunction>,
}

impl Procedure {
    /// Create a new function.
    pub fn new<S: Into<Symbol>, F: 'static + Send + Sync + Fn(&[Val]) -> Result<Val>>(
        name: Option<S>,
        f: F,
    ) -> Arc<Procedure> {
        let f = Box::new(f);
        let name = name.map(|n| n.into());
        Arc::new(Procedure { name, f })
    }

    pub fn name(&self) -> Option<&Symbol> {
        self.name.as_ref()
    }
}

impl Procedure {
    /// Evaluate the function with the given arguments.
    pub fn eval(&self, args: &[Val]) -> Result<Val> {
        self.f.as_ref()(args)
    }
}

impl PartialEq for Procedure {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.f.as_ref(), other.f.as_ref())
    }
}

impl std::fmt::Debug for Procedure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = self.name.as_ref().map(|s| s.as_str()).unwrap_or("_");
        f.debug_struct("Function")
            .field("name", &name)
            .finish_non_exhaustive()
    }
}

impl std::fmt::Display for Procedure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.name.as_ref() {
            Some(name) => write!(f, "<proc {name}>", name = name.as_str()),
            None => write!(f, "<proc _>"),
        }
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

    #[test]
    fn functions_pointing_to_same_impl_are_eq() {
        let noop = |_: &[Val]| Ok(Val::Void);
        let a = Procedure::new(Some("noop"), noop);
        let b = Procedure::new(Some("noop"), noop);
        assert_eq!(a, b);
    }

    #[test]
    fn functions_pointing_to_different_impl_are_not_eq() {
        // Note: The return values are different so that the Rust optimizer doesn't unify their
        // implementations.
        let a = Procedure::new(Some("void"), |_: &[Val]| Ok(Val::Void));
        let b = Procedure::new(Some("make-string"), |_: &[Val]| {
            Ok(Val::String(Arc::new("".to_string())))
        });
        assert_ne!(a, b);
    }
}
