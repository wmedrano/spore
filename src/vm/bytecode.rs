use anyhow::{bail, Result};

use crate::parser::ast::Ast;

use super::{
    sexp::Sexp,
    types::{Symbol, Val},
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
        Self::with_sexpr(&Sexp::with_ast(ast)?)
    }

    pub fn with_sexpr(sexpr: &Sexp) -> Result<Vec<ByteCode>> {
        let mut res = Vec::new();
        Self::with_sexpr_impl(sexpr.as_val(), &mut res)?;
        Ok(res)
    }

    fn with_sexpr_impl(expr: &Val, res: &mut Vec<ByteCode>) -> Result<()> {
        match expr {
            v @ Val::Void
            | v @ Val::String(_)
            | v @ Val::Bool(_)
            | v @ Val::Number(_)
            | v @ Val::Proc(_) => res.push(ByteCode::PushVal(v.clone())),
            Val::List(l) => Self::build_proc(l.as_slice(), res)?,
            Val::Symbol(s) => Self::build_symbol_bytecode(s.as_str(), res)?,
        }
        Ok(())
    }

    fn build_proc(expr: &[Val], res: &mut Vec<ByteCode>) -> Result<()> {
        match expr {
            [] => bail!("procedure call requires at least 1 argument"),
            [proc, args @ ..] => match proc {
                Val::Symbol(s) if s.as_str() == "if" => {
                    let mut args = args.iter();
                    let pred = args.next().ok_or_else(|| {
                        anyhow::anyhow!("if expression expected a predicate and true value")
                    })?;
                    let t_val = args
                        .next()
                        .ok_or_else(|| anyhow::anyhow!("if expression expected a true value"))?;
                    let f_val = args.next().unwrap_or(&Val::Void);
                    Self::build_if_bytecode(pred, t_val, f_val, res)?;
                }
                _ => {
                    Self::with_sexpr_impl(proc, res)?;
                    for arg in args {
                        Self::with_sexpr_impl(arg, res)?;
                    }
                    res.push(ByteCode::Eval(expr.len()));
                }
            },
        }
        Ok(())
    }

    /// Build the bytecode for the symbol and push the contents onto `res.`
    fn build_symbol_bytecode(ident: &str, res: &mut Vec<ByteCode>) -> Result<()> {
        // Handle quoted identifiers which are left as raw symbols.
        if let Some(sym) = ident.strip_prefix('\'') {
            res.push(ByteCode::PushVal(Symbol::from(sym.trim()).into()));
            return Ok(());
        }
        res.extend([
            ByteCode::PushVal(Vm::singleton().get_value("%get-sym").unwrap()),
            ByteCode::PushVal(Symbol::from(ident).into()),
            ByteCode::Eval(2),
        ]);
        Ok(())
    }

    /// Build the bytecode for an if statement and push the contents onto `res`.
    fn build_if_bytecode(pred: &Val, t: &Val, f: &Val, res: &mut Vec<ByteCode>) -> Result<()> {
        Self::with_sexpr_impl(pred, res)?;
        let jump_if_idx = res.len();
        res.push(ByteCode::JumpIf(0));
        Self::with_sexpr_impl(f, res)?;
        let jump_idx = res.len();
        res.push(ByteCode::Jump(0));
        Self::with_sexpr_impl(t, res)?;
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
