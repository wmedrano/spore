use crate::parser::ast::{Ast, AstLeaf};

use super::{
    types::{Number, Symbol, Val},
    Vm,
};

/// Contains an expression that can be evaluated.
pub struct Expr<T: AsRef<Val>>(T);

impl Expr<Val> {
    /// Evaluate a string.
    pub fn eval_str(s: &str) -> Vec<Val> {
        Ast::from_sexp_str(s)
            .unwrap()
            .iter()
            .map(Expr::from_ast)
            .map(|expr| expr.eval())
            .collect()
    }

    /// Converts an AST into an expression.
    pub fn from_ast(ast: &Ast) -> Expr<Val> {
        let expr = match ast {
            Ast::Leaf(n) => match &n.item {
                AstLeaf::Identifier(ident) => Symbol::from(ident.as_str()).into(),
                AstLeaf::String(s) => s.clone().into(),
                AstLeaf::Float(f) => Number::Float(*f).into(),
                AstLeaf::Int(i) => Number::Int(*i).into(),
            },
            Ast::Tree(children) => {
                let list: Vec<Val> = children.iter().map(Self::from_ast).map(|bc| bc.0).collect();
                list.into()
            }
        };
        Expr(expr)
    }
}

impl<T: AsRef<Val>> Expr<T> {
    /// Evaluate then expression and return the result.
    fn eval(&self) -> Val {
        let vm = Vm::singleton();
        let eval_list = |exprs: &[Val]| -> Val {
            match exprs {
                [] => todo!(),
                [func, args @ ..] => {
                    let f = match func {
                        Val::Symbol(f_symbol) => {
                            match vm.get_value(f_symbol).expect("Symbol not found") {
                                Val::Function(f) => f,
                                _ => todo!(),
                            }
                        }
                        Val::Function(f) => f.clone(),
                        _ => todo!("provide error message"),
                    };
                    let evaled_args = Vec::from_iter(args.iter().map(|expr| Expr(expr).eval()));
                    f.eval(&evaled_args)
                }
            }
        };

        match self.0.as_ref() {
            Val::Void | Val::String(_) | Val::Symbol(_) | Val::Number(_) | Val::Function(_) => {
                self.0.as_ref().clone()
            }
            Val::List(exprs) => eval_list(exprs.as_slice()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn expression_can_eval() {
        assert_eq!(
            Expr::eval_str("(+ 1 2 )"),
            vec![Val::Number(Number::Int(3))],
        );
    }

    #[test]
    fn nested_expressions_are_evaled() {
        assert_eq!(
            Expr::eval_str("(+ (* 2 3) (* 4 5))"),
            vec![Val::Number(Number::Int(26))],
        );
    }

    #[test]
    fn multiple_expressions_return_multiple_values() {
        assert_eq!(
            Expr::eval_str("(+ 1 2) (* 0.5 6.0)"),
            vec![Val::Number(Number::Int(3)), Val::Number(Number::Float(3.0))],
        );
    }
}
