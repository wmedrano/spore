use std::sync::Arc;

use anyhow::{bail, Result};

use crate::parser::ast::{Ast, AstLeaf};

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

    fn build_leaf_bytecode(leaf: &AstLeaf, res: &mut Vec<ByteCode>) -> Result<()> {
        let v = match leaf {
            AstLeaf::Identifier(x) => match Vm::singleton().get_value(x) {
                _ if x.as_str().starts_with('\'') => {
                    Val::Symbol(Symbol::from(x.as_str()[1..].trim()))
                }
                Some(x) => x,
                None => {
                    res.extend([
                        ByteCode::PushVal(Vm::singleton().get_value("%get-sym").unwrap()),
                        ByteCode::PushVal(Symbol::from(x.as_str()).into()),
                        ByteCode::Eval(2),
                    ]);
                    return Ok(());
                }
            },
            AstLeaf::String(x) => Val::String(Arc::new(x.clone())),
            AstLeaf::Float(x) => Val::Number(Number::Float(*x)),
            AstLeaf::Int(x) => Val::Number(Number::Int(*x)),
            AstLeaf::Bool(x) => Val::Bool(*x),
        };
        res.push(ByteCode::PushVal(v));
        Ok(())
    }

    fn build_if_bytecode(expr: &[Ast], res: &mut Vec<ByteCode>) -> Result<()> {
        match expr {
            [_, pred, t] => {
                Self::with_ast_impl(pred, res)?;
                let jump_if_idx = res.len();
                res.push(ByteCode::JumpIf(0));
                res.push(ByteCode::PushVal(Val::Void));
                let jump_idx = res.len();
                res.push(ByteCode::Jump(0));
                Self::with_ast_impl(t, res)?;
                let end = res.len();
                res[jump_if_idx] = ByteCode::JumpIf(jump_idx - jump_if_idx);
                res[jump_idx] = ByteCode::Jump(end - jump_idx - 1);
            }
            [_, pred, t, f] => {
                Self::with_ast_impl(pred, res)?;
                let jump_if_idx = res.len();
                res.push(ByteCode::JumpIf(0));
                Self::with_ast_impl(f, res)?;
                let jump_idx = res.len();
                res.push(ByteCode::Jump(0));
                Self::with_ast_impl(t, res)?;
                let end = res.len();
                res[jump_if_idx] = ByteCode::JumpIf(jump_idx - jump_if_idx);
                res[jump_idx] = ByteCode::Jump(end - jump_idx - 1);
            }
            _ => bail!(
                "malformed if statement, if requires the form (if pred-expr true-expr false-expr)"
            ),
        };
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
