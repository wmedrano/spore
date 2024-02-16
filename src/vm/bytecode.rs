use anyhow::{bail, Result};

use crate::parser::{
    ast::{Ast, AstLeaf},
    token::Token,
};

use super::{
    types::{Number, Symbol, Val},
    Vm,
};

#[derive(Clone, Debug)]
pub enum ByteCode {
    /// Push a new value to the stack.
    PushVal(Val),
    /// Evaluate the top of the stack.
    Eval(usize),
    /// Jump a number of bytecodes if the result at the top of the stack is false.
    JumpIf(usize),
    /// Jump ahead by the given number of bytecode instructions.
    Jump(usize),
}

impl ByteCode {
    /// Create new bytecode instructions from an AST.
    pub fn with_ast(ast: &Ast) -> Result<Vec<ByteCode>> {
        let mut ret = Vec::new();
        Self::with_ast_impl(ast, &mut ret)?;
        Ok(ret)
    }

    fn with_ast_impl(ast: &Ast, res: &mut Vec<ByteCode>) -> Result<()> {
        match ast {
            Ast::Leaf(l) => Self::build_leaf_bytecode(&l.item, res)?,
            Ast::Tree(children) => {
                if children
                    .iter()
                    .next()
                    .map(|t| matches!(t, Ast::Leaf(l) if l.item == AstLeaf::Identifier("if".to_string())))
                    .unwrap_or(false)
                {
                    return Self::build_if_bytecode(children, res);
                }
                for child in children.iter() {
                    Self::with_ast_impl(child, res)?;
                }
                res.push(ByteCode::Eval(children.len()));
            }
        }
        Ok(())
    }

    // Build the bytecode for `leaf` and push the contents onto `res`.
    fn build_leaf_bytecode(leaf: &AstLeaf, res: &mut Vec<ByteCode>) -> Result<()> {
        match leaf {
            AstLeaf::Identifier(x) => Self::build_identifier_bytecode(x, res)?,
            AstLeaf::String(x) => res.push(ByteCode::PushVal(x.clone().into())),
            AstLeaf::Float(x) => res.push(ByteCode::PushVal(Number::Float(*x).into())),
            AstLeaf::Int(x) => res.push(ByteCode::PushVal(Number::Int(*x).into())),
            AstLeaf::Bool(x) => res.push(ByteCode::PushVal(Val::Bool(*x))),
        };
        Ok(())
    }

    /// Build the bytecode for the identifier and push the contents onto `res.`
    fn build_identifier_bytecode(ident: &str, res: &mut Vec<ByteCode>) -> Result<()> {
        // Handle quoted identifiers which are left as raw symbols.
        if let Some(sym) = ident.strip_prefix('\'') {
            res.push(ByteCode::PushVal(Symbol::from(sym.trim()).into()));
            return Ok(());
        }
        // Optimization: Attempt to find the value of the identifier. If the identifier's value
        // could not be resolved, fall back to determining it at runtime.
        match Vm::singleton().get_value(ident) {
            Some(v) => res.push(ByteCode::PushVal(v)),
            None => {
                res.extend([
                    ByteCode::PushVal(Vm::singleton().get_value("%get-sym").unwrap()),
                    ByteCode::PushVal(Symbol::from(ident).into()),
                    ByteCode::Eval(2),
                ]);
            }
        }
        Ok(())
    }

    /// Build the bytecode for an if statement and push the contents onto `res`.
    fn build_if_bytecode(expr: &[Ast], res: &mut Vec<ByteCode>) -> Result<()> {
        assert!(
            matches!(&expr[0], Ast::Leaf(Token {item: AstLeaf::Identifier(ident),..}) if ident == "if")
        );
        let (pred, t, maybe_f) = match expr {
            [_, pred, t, f] => (pred, t, Some(f)),
            [_, pred, t] => (pred, t, None),
            _ => bail!(
                "malformed if statement, if requires the form (if pred-expr true-expr false-expr)"
            ),
        };
        Self::with_ast_impl(pred, res)?;
        let jump_if_idx = res.len();
        res.push(ByteCode::JumpIf(0));
        match maybe_f {
            Some(f) => Self::with_ast_impl(f, res)?,
            None => res.push(ByteCode::PushVal(Val::Void)),
        };
        let jump_idx = res.len();
        res.push(ByteCode::Jump(0));
        Self::with_ast_impl(t, res)?;
        let end = res.len();
        res[jump_if_idx] = ByteCode::JumpIf(jump_idx - jump_if_idx);
        res[jump_idx] = ByteCode::Jump(end - jump_idx - 1);
        Ok(())
    }
}

impl std::fmt::Display for ByteCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ByteCode::PushVal(v) => write!(f, "push value {v}"),
            ByteCode::Eval(n) => write!(f, "evaluate last {n}"),
            ByteCode::JumpIf(n) => write!(f, "jump {n} if true"),
            ByteCode::Jump(n) => write!(f, "jump {n}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn byte_code_size_is_small() {
        assert_eq!(
            std::mem::size_of::<ByteCode>(),
            2 * std::mem::size_of::<usize>()
        )
    }
}
