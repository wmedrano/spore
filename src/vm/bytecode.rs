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
        let mut res = Vec::new();
        Self::with_ast_impl(ast, &mut res)?;
        Ok(res)
    }

    fn with_ast_impl(ast: &Ast, res: &mut Vec<ByteCode>) -> Result<()> {
        match ast {
            Ast::Leaf(l) => Self::leaf_to_bytecode(&l.item, res)?,
            Ast::Tree(children) => match children.as_slice() {
                [] => bail!("found empty expression"),
                [Ast::Leaf(Token {
                    item: AstLeaf::If, ..
                }), args @ ..] => match args {
                    [pred, t_val, maybe_f_val @ ..] => {
                        Self::with_ast_impl(pred, res)?;
                        let t_bytecode = Self::with_ast(t_val)?;
                        let f_bytecode = match maybe_f_val {
                            [] => vec![ByteCode::PushVal(Val::Void)],
                            [f_val] => Self::with_ast(f_val)?,
                            rest => bail!("expected single false expression but found {}", rest.len()),
                        };
                        res.push(ByteCode::JumpIf(f_bytecode.len() + 1));
                        res.extend(f_bytecode);
                        res.push(ByteCode::Jump(t_bytecode.len()));
                        res.extend(t_bytecode);
                    }
                    _ => bail!("if statement requires 3 (condition-bool true-expression false-expression) args but found {}", args.len()),
                },
                vals => {
                    for v in vals {
                        Self::with_ast_impl(v, res)?;
                    }
                    res.push(ByteCode::Eval(vals.len()));
                },
            },
        }
        Ok(())
    }

    fn leaf_to_bytecode(l: &AstLeaf, res: &mut Vec<ByteCode>) -> Result<()> {
        match l {
            AstLeaf::If => bail!("empty if is not a valid expression"),
            AstLeaf::Identifier(x) => res.extend([
                ByteCode::PushVal(Vm::singleton().get_value("%get-sym").unwrap()),
                ByteCode::PushVal(Symbol::from(x.clone()).into()),
                ByteCode::Eval(2),
            ]),
            AstLeaf::Symbol(x) => res.push(ByteCode::PushVal(Symbol::from(x.clone()).into())),
            AstLeaf::String(x) => res.push(ByteCode::PushVal(x.clone().into())),
            AstLeaf::Float(x) => res.push(ByteCode::PushVal(Number::Float(*x).into())),
            AstLeaf::Int(x) => res.push(ByteCode::PushVal(Number::Int(*x).into())),
            AstLeaf::Bool(x) => res.push(ByteCode::PushVal((*x).into())),
        }
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
