use std::sync::Arc;

/// Contains a single value.
#[derive(Clone, Debug, PartialEq)]
pub enum Val {
    Void,
    String(Arc<String>),
    Symbol(Symbol),
    Number(Number),
    Function(Arc<Function>),
    List(Box<Vec<Val>>),
}

impl AsRef<Val> for Val {
    fn as_ref(&self) -> &Val {
        &self
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
impl_enum_from!(Val, Function => Function);
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

/// A function.
#[repr(transparent)]
pub struct Function {
    f: Box<dyn Send + Sync + Fn(&[Val]) -> Val>,
}

impl Function {
    pub fn new(f: impl 'static + Send + Sync + Fn(&[Val]) -> Val) -> Arc<Function> {
        Arc::new(Function { f: Box::new(f) })
    }
}

impl Function {
    /// Evaluate the function with the given arguments.
    pub fn eval(&self, args: &[Val]) -> Val {
        self.f.as_ref()(args)
    }
}

impl PartialEq for Function {
    fn eq(&self, other: &Self) -> bool {
        (self.f.as_ref() as *const _) == (other.f.as_ref() as *const _)
    }
}

impl std::fmt::Debug for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ptr: *const _ = self.f.as_ref();
        f.debug_struct("Function")
            .field("fn_ptr", &ptr)
            .finish_non_exhaustive()
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
        let noop = |_: &[Val]| Val::Void;
        let a = Function::new(noop);
        let b = Function::new(noop);
        assert_eq!(a, b);
    }

    #[test]
    fn functions_pointing_to_different_impl_are_not_eq() {
        // Note: The return values are different so that the Rust optimizer doesn't unify their
        // implementations.
        let a = Function::new(|_: &[Val]| Val::Void);
        let b = Function::new(|_: &[Val]| Val::String(Arc::new("".to_string())));
        assert_ne!(a, b);
    }
}
