use std::sync::Arc;

use crate::parser::{Ast, AstRoot};

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

impl Val {
    // Create a new value from an AST.
    pub fn from_ast(ast: &Ast) -> Val {
        match ast {
            Ast::Root(n) => match &n.item {
                AstRoot::Identifier(ident) => Symbol(ident.clone().into()).into(),
                AstRoot::String(s) => s.clone().into(),
                AstRoot::Float(f) => Number::Float(*f).into(),
                AstRoot::Int(i) => Number::Int(*i).into(),
            },
            Ast::Tree(children) => {
                let list: Vec<_> = children.iter().map(Val::from_ast).collect();
                list.into()
            }
        }
    }
}

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
#[derive(Clone, Debug, PartialEq)]
pub struct Symbol(Arc<String>);

/// A function.
#[repr(transparent)]
pub struct Function {
    f: Box<dyn Fn(&[Val]) -> Val>,
}

impl<F: 'static + Fn(&[Val]) -> Val> From<F> for Function {
    fn from(f: F) -> Function {
        Function { f: Box::new(f) }
    }
}

impl Function {
    /// Evaluate the function with the given arguments.
    pub fn eval(&self, args: &[Val]) -> Val {
        self.f.as_ref()(args)
    }
}

impl PartialEq for Function {
    fn eq(&self, _: &Self) -> bool {
        false
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

    fn val_from_str(s: &str) -> Val {
        let ast = Ast::from_str(s).unwrap();
        Val::from_ast(&Ast::Tree(ast))
    }

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
    fn val_from_ast() {
        assert_eq!(
            val_from_str("(1 2.0 \"hello\") ()"),
            Val::List(Box::new(vec![
                vec![
                    Number::Int(1).into(),
                    Number::Float(2.0).into(),
                    "hello".to_string().into(),
                ]
                .into(),
                Val::List(Box::new(vec![]))
            ]))
        );
    }
}
