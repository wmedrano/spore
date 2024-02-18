use anyhow::{bail, Result};

use crate::parser::{
    ast::{Ast, AstLeaf},
    token::Token,
};

use super::{
    bytecode::OpCode,
    types::{GenericProcedure, Number, Procedure, Symbol, Val},
    Vm,
};

pub struct ByteCodeProc {
    bytecode: Vec<OpCode>,
}

impl GenericProcedure for ByteCodeProc {
    fn eval(&self, stack: &mut Vec<Val>, arg_count: usize) -> Result<Val> {
        let end = stack.len();
        let start = end - arg_count;
        Vm::singleton().eval_bytecode(&self.bytecode, stack)?;
        let val = stack.pop().unwrap();
        stack.truncate(start);
        Ok(val)
    }
}

impl ByteCodeProc {
    pub fn with_ast(ast: &Ast) -> Result<ByteCodeProc> {
        let mut bytecode = Vec::new();
        ast_to_bytecode_impl(ast, &mut bytecode)?;
        bytecode.shrink_to_fit();
        Ok(ByteCodeProc { bytecode })
    }

    pub fn instructions(&self) -> &[OpCode] {
        self.bytecode.as_slice()
    }
}

fn ast_to_bytecode_impl(ast: &Ast, res: &mut Vec<OpCode>) -> Result<()> {
    match ast {
        Ast::Leaf(l) => leaf_to_bytecode(&l.item, res)?,
        Ast::Tree(children) => match children.as_slice() {
            [] => bail!("found empty expression"),
            [Ast::Leaf(Token { item, .. }), args @ ..] => match item {
                AstLeaf::If => if_to_bytecode(args, res)?,
                AstLeaf::Lambda => lambda_to_bytecode(args, res)?,
                AstLeaf::Define => define_to_bytecode(args, res)?,
                AstLeaf::Identifier(_) => {
                    for c in children {
                        ast_to_bytecode_impl(c, res)?;
                    }
                    res.push(OpCode::Eval(children.len()));
                }
                v @ AstLeaf::Symbol(_)
                | v @ AstLeaf::String(_)
                | v @ AstLeaf::Float(_)
                | v @ AstLeaf::Int(_)
                | v @ AstLeaf::Bool(_) => bail!("cannot evaluate object {:?}", v),
            },
            vals => {
                for v in vals {
                    ast_to_bytecode_impl(v, res)?;
                }
                res.push(OpCode::Eval(vals.len()));
            }
        },
    }
    Ok(())
}

fn leaf_to_bytecode(l: &AstLeaf, res: &mut Vec<OpCode>) -> Result<()> {
    match l {
        AstLeaf::If => bail!("empty if is not a valid expression"),
        AstLeaf::Lambda => bail!("empty lambda is not a valid expression"),
        AstLeaf::Define => bail!("empty define is not a valid expression"),
        AstLeaf::Identifier(x) => res.extend([
            OpCode::PushVal(Vm::singleton().get_value("%get-sym").unwrap()),
            OpCode::PushVal(Symbol::from(x.clone()).into()),
            OpCode::Eval(2),
        ]),
        AstLeaf::Symbol(x) => res.push(OpCode::PushVal(Symbol::from(x.clone()).into())),
        AstLeaf::String(x) => res.push(OpCode::PushVal(x.clone().into())),
        AstLeaf::Float(x) => res.push(OpCode::PushVal(Number::Float(*x).into())),
        AstLeaf::Int(x) => res.push(OpCode::PushVal(Number::Int(*x).into())),
        AstLeaf::Bool(x) => res.push(OpCode::PushVal((*x).into())),
    }
    Ok(())
}

fn lambda_to_bytecode(args: &[Ast], res: &mut Vec<OpCode>) -> Result<()> {
    match args {
        [] => bail!("found (lambda) but lambda must have form (lambda (..args) exprs)"),
        [lambda_args, exprs @ ..] => {
            // Validate
            match lambda_args {
                Ast::Leaf(_) => bail!("first argument to lambda must be a list of arguments"),
                Ast::Tree(args) => {
                    if !args.is_empty() {
                        bail!("lambda with arguments are not yet supported");
                    }
                }
            };
            // Build bytecode
            let mut lambda_bytecode = Vec::new();
            for expr in exprs {
                ast_to_bytecode_impl(expr, &mut lambda_bytecode)?;
            }
            let bytecode_proc = Procedure::new(
                Some("lambda"),
                ByteCodeProc {
                    bytecode: lambda_bytecode,
                },
            );
            res.push(OpCode::PushVal(Val::Proc(bytecode_proc)));
        }
    };
    Ok(())
}

fn define_to_bytecode(args: &[Ast], res: &mut Vec<OpCode>) -> Result<()> {
    match args {
        [sym, expr] => {
            let sym = match sym {
                Ast::Leaf(Token {
                    item: AstLeaf::Identifier(ident),
                    ..
                }) => Symbol::from(ident.as_str()),
                ast => bail!("define must be bound to an identifier but found {:?}", ast),
            };
            res.extend([
                OpCode::PushVal(Vm::singleton().get_value("%define-sym").unwrap()),
                OpCode::PushVal(sym.into()),
            ]);
            ast_to_bytecode_impl(expr, res)?;
            res.push(OpCode::Eval(3));
        }
        _ => bail!("define requires 2 args but found {}", args.len()),
    };
    Ok(())
}

fn if_to_bytecode(args: &[Ast], res: &mut Vec<OpCode>) -> Result<()> {
    match args {
            [pred, t_val, maybe_f_val @ ..] => {
                ast_to_bytecode_impl(pred, res)?;
                let t_bytecode = ByteCodeProc::with_ast(t_val)?.bytecode;
                let f_bytecode = match maybe_f_val {
                    [] => vec![OpCode::PushVal(Val::Void)],
                    [f_val] => ByteCodeProc::with_ast(f_val)?.bytecode,
                    rest => bail!("expected single false expression but found {}", rest.len()),
                };
                res.push(OpCode::JumpIf(f_bytecode.len() + 1));
                res.extend(f_bytecode);
                res.push(OpCode::Jump(t_bytecode.len()));
                res.extend(t_bytecode);
            }
            _ => bail!("if statement requires 3 (condition-expr true-expression false-expression) args but found {}", args.len()),
        };
    Ok(())
}
