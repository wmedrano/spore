use anyhow::{bail, Result};

use crate::parser::ast::{Ast, AstLeaf};

use super::{
    types::{Number, Symbol, Val},
    Vm,
};

/// Contains an expression that can be evaluated.
pub struct Expr<T: AsRef<Val>>(T);

impl Expr<Val> {
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
    pub fn eval(&self) -> Result<Val> {
        let vm = Vm::singleton();
        let eval_list = |exprs: &[Val]| -> Result<Val> {
            match exprs {
                [] => bail!("Attempted to evaluate empty expression."),
                [func, args @ ..] => {
                    let f = match func {
                        Val::Symbol(f_symbol) => match vm.get_value(f_symbol) {
                            Some(Val::Proc(f)) => f,
                            Some(v) => bail!("Can not evaluate value {v}."),
                            None => bail!("Symbol {} not found.", f_symbol.as_str()),
                        },
                        Val::Proc(f) => f.clone(),
                        v => bail!("Can not evaluate value {v}."),
                    };
                    let evaled_args: Result<Vec<Val>> =
                        args.iter().map(|expr| Expr(expr).eval()).collect();
                    f.eval(evaled_args?.as_slice())
                }
            }
        };

        match self.0.as_ref() {
            Val::Void | Val::String(_) | Val::Symbol(_) | Val::Number(_) | Val::Proc(_) => {
                Ok(self.0.as_ref().clone())
            }
            Val::List(exprs) => eval_list(exprs.as_slice()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn eval_str(s: &str) -> Vec<Val> {
        Ast::from_sexp_str(s)
            .unwrap()
            .into_iter()
            .map(|ast| Expr::from_ast(&ast))
            .map(|expr| expr.eval().unwrap())
            .collect()
    }

    #[test]
    fn expression_can_eval() {
        assert_eq!(eval_str("(+ 1 2 )"), vec![Val::Number(Number::Int(3))],);
    }

    #[test]
    fn nested_expressions_are_evaled() {
        assert_eq!(
            eval_str("(+ (* 2 3) (* 4 5))"),
            vec![Val::Number(Number::Int(26))],
        );
    }

    #[test]
    fn multiple_expressions_return_multiple_values() {
        assert_eq!(
            eval_str("(+ 1 2) (* 0.5 6.0)"),
            vec![Val::Number(Number::Int(3)), Val::Number(Number::Float(3.0))],
        );
    }
}
