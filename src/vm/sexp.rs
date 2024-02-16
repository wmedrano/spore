use anyhow::Result;

use crate::parser::ast::{Ast, AstLeaf};

use super::types::{Number, Symbol, Val};

pub struct Sexp(Val);

impl Sexp {
    /// Create an S-Expression from an AST.
    pub fn with_ast(ast: &Ast) -> Result<Sexp> {
        Self::with_ast_impl(ast).map(Sexp)
    }

    /// Get the S-expression as a Spore value.
    pub fn as_val(&self) -> &Val {
        &self.0
    }

    fn with_ast_impl(ast: &Ast) -> Result<Val> {
        let expr: Val = match ast {
            Ast::Leaf(l) => match &l.item {
                AstLeaf::Identifier(ident) => Symbol::from(ident.clone()).into(),
                AstLeaf::String(v) => v.clone().into(),
                AstLeaf::Float(v) => Number::Float(*v).into(),
                AstLeaf::Int(v) => Number::Int(*v).into(),
                AstLeaf::Bool(v) => Val::Bool(*v),
            },
            Ast::Tree(children) => {
                let maybe_exprs: Result<Vec<_>> =
                    children.iter().map(Self::with_ast_impl).collect();
                Val::List(Box::new(maybe_exprs?))
            }
        };
        Ok(expr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ast_can_parse_as_sexpression() {
        let ast = Ast::from_sexp_str("(hello \"world\" 1.0 2 true)")
            .unwrap()
            .into_iter()
            .next()
            .unwrap();
        assert_eq!(
            Sexp::with_ast(&ast).unwrap().as_val(),
            &Val::List(
                vec![
                    Symbol::from("hello").into(),
                    String::from("world").into(),
                    Number::Float(1.0).into(),
                    Number::Int(2).into(),
                    Val::Bool(true),
                ]
                .into()
            )
        );
    }
}
