use std::collections::HashMap;

use anyhow::{bail, Result};

use crate::parser::{
    ast::{Ast, AstLeaf},
    token::Token,
};

use super::{
    environment::Environment,
    types::{instruction::Instruction, proc::ByteCodeProc, symbol::Symbol, Number, Val},
};

/// Compiles Asts into `ByteCodeProc` objects.
pub struct Compiler<'a> {
    env: &'a mut Environment,
    name: String,
    symbol_to_idx: HashMap<String, usize>,
    opcodes: Vec<Instruction>,
}

impl<'a> Compiler<'a> {
    /// Create a new compiler.
    pub fn new(name: &str, env: &'a mut Environment) -> Compiler<'a> {
        Compiler::with_args(name, env, std::iter::empty())
    }

    /// Create a new compiler with the given arguments at the base of the stack.
    pub fn with_args(
        name: &str,
        env: &'a mut Environment,
        args: impl Iterator<Item = String>,
    ) -> Compiler<'a> {
        let symbol_to_idx = args.enumerate().map(|(idx, sym)| (sym, idx)).collect();
        Compiler::with_symbols(name, env, symbol_to_idx)
    }

    /// Create a new compiler where the base of the stack contains the values from `symbol_to_idx`.
    pub fn with_symbols(
        name: &'_ str,
        env: &'a mut Environment,
        symbol_to_idx: HashMap<String, usize>,
    ) -> Compiler<'a> {
        let name = name.to_string();
        Compiler {
            env,
            name,
            symbol_to_idx,
            opcodes: Vec::new(),
        }
    }

    /// Compile `ast` onto the current context and return the callable `ByteCodeProc`.
    pub fn compile_and_finalize(self, ast: &Ast) -> Result<ByteCodeProc> {
        let mut c = self;
        c.compile(ast)?;
        Ok(c.finalize())
    }

    /// Finalize the current compilation and return the result as a callable `ByteCodeProc`.
    fn finalize(self) -> ByteCodeProc {
        let mut bytecode = self.opcodes;
        if bytecode.is_empty() {
            bytecode.push(Instruction::PushVal(Val::Void));
        }
        for bc in bytecode.iter_mut() {
            if let Instruction::GetVal(sym) = bc {
                if let Some(val) = self.env.globals.get(sym) {
                    *bc = Instruction::PushVal(val.clone());
                }
            }
        }
        bytecode.shrink_to_fit();
        let arg_count = self
            .symbol_to_idx
            .values()
            .map(|idx| idx + 1)
            .max()
            .unwrap_or(0);
        ByteCodeProc {
            name: self.name.clone(),
            arg_count,
            bytecode,
        }
    }

    /// Compile `ast` onto the current context.
    fn compile(&mut self, ast: &Ast) -> Result<()> {
        match ast {
            Ast::Leaf(l) => self.leaf_to_bytecode(&l.item)?,
            Ast::Tree(children) => match children.as_slice() {
                [] => bail!("found empty expression"),
                [Ast::Leaf(Token { item, .. }), args @ ..] => match item {
                    AstLeaf::If => self.if_to_bytecode(args)?,
                    AstLeaf::Lambda => self.lambda_to_bytecode(args)?,
                    AstLeaf::Define => self.define_to_bytecode(args)?,
                    AstLeaf::Identifier(_) => {
                        for c in children {
                            self.compile(c)?;
                        }
                        self.opcodes.push(Instruction::Eval(children.len()));
                    }
                    v @ AstLeaf::Symbol(_)
                    | v @ AstLeaf::String(_)
                    | v @ AstLeaf::Float(_)
                    | v @ AstLeaf::Int(_)
                    | v @ AstLeaf::Bool(_) => bail!("cannot evaluate object {:?}", v),
                },
                vals => {
                    for v in vals {
                        self.compile(v)?;
                    }
                    self.opcodes.push(Instruction::Eval(vals.len()));
                }
            },
        }
        Ok(())
    }

    fn leaf_to_bytecode(&mut self, l: &AstLeaf) -> Result<()> {
        match l {
            AstLeaf::If => bail!("empty if is not a valid expression"),
            AstLeaf::Lambda => bail!("empty lambda is not a valid expression"),
            AstLeaf::Define => bail!("empty define is not a valid expression"),
            AstLeaf::Identifier(x) => match self.symbol_to_idx.get(x) {
                Some(idx) => self.opcodes.push(Instruction::GetArg(*idx)),
                None => {
                    self.opcodes.push(Instruction::GetVal(x.as_str().into()));
                }
            },
            AstLeaf::Symbol(x) => self
                .opcodes
                .push(Instruction::PushVal(Symbol::from(x.clone()).into())),
            AstLeaf::String(_) => unimplemented!("Strings are not yet supported."),
            AstLeaf::Float(x) => self
                .opcodes
                .push(Instruction::PushVal(Number::Float(*x).into())),
            AstLeaf::Int(x) => self
                .opcodes
                .push(Instruction::PushVal(Number::Int(*x).into())),
            AstLeaf::Bool(x) => self.opcodes.push(Instruction::PushVal((*x).into())),
        }
        Ok(())
    }

    fn lambda_to_bytecode(&mut self, args: &[Ast]) -> Result<()> {
        match args {
            [] => bail!("found (lambda) but lambda must have form (lambda (..args) exprs)"),
            [lambda_args, exprs @ ..] => {
                // Validate
                let symbol_to_idx = match lambda_args {
                    Ast::Leaf(_) => bail!("first argument to lambda must be a list of arguments"),
                    Ast::Tree(args) => symbol_to_idx(args.as_slice())?,
                };
                // Build bytecode
                let mut lambda_compiler = Compiler {
                    env: self.env,
                    name: self.name.clone(),
                    symbol_to_idx,
                    opcodes: Vec::new(),
                };
                for expr in exprs {
                    lambda_compiler.compile(expr)?;
                }
                let bytecode = lambda_compiler.finalize();
                self.opcodes.push(Instruction::PushVal(bytecode.into()));
            }
        };
        Ok(())
    }

    fn define_to_bytecode(&mut self, args: &[Ast]) -> Result<()> {
        match args {
            [sym, expr] => {
                let sym = match sym {
                    Ast::Leaf(Token {
                        item: AstLeaf::Identifier(ident),
                        ..
                    }) => Symbol::from(ident.as_str()),
                    ast => bail!("define must be bound to an identifier but found {:?}", ast),
                };
                let mut tmp_name = sym.0.to_string();
                std::mem::swap(&mut tmp_name, &mut self.name);
                self.compile(expr)?;
                std::mem::swap(&mut tmp_name, &mut self.name);
                self.opcodes.push(Instruction::SetVal(sym));
            }
            _ => bail!("define requires 2 args but found {}", args.len()),
        };
        Ok(())
    }

    fn if_to_bytecode(&mut self, args: &[Ast]) -> Result<()> {
        match args {
            [pred, t_val, maybe_f_val @ ..] => {
                self.compile(pred)?;
                let t_bytecode = Compiler::with_symbols("", self.env, self.symbol_to_idx.clone()).compile_and_finalize(t_val)?.bytecode;
                let f_bytecode = match maybe_f_val {
                    [] => vec![Instruction::PushVal(Val::Void)],
                    [f_val] => Compiler::with_symbols("", self.env, self.symbol_to_idx.clone()).compile_and_finalize(f_val)?.bytecode.to_vec(),
                    rest => bail!("expected single false expression but found {}", rest.len()),
                };
                self.opcodes.push(Instruction::JumpIf(f_bytecode.len() + 1));
                self.opcodes.extend(f_bytecode);
                self.opcodes.push(Instruction::Jump(t_bytecode.len()));
                self.opcodes.extend(t_bytecode.iter().cloned());
            }
            _ => bail!("if statement requires 3 (condition-expr true-expression false-expression) args but found {}", args.len()),
        };
        Ok(())
    }
}

fn symbol_to_idx(list: &[Ast]) -> Result<HashMap<String, usize>> {
    let mut res = HashMap::new();
    for (idx, element) in list.iter().enumerate() {
        match element {
            Ast::Leaf(l) => match &l.item {
                AstLeaf::Identifier(ident) => res.insert(ident.clone(), idx),
                l => bail!("expected identifier but found {l:?}"),
            },
            Ast::Tree(_) => bail!("expected identifier but found expression"),
        };
    }
    Ok(res)
}

#[cfg(test)]
mod tests {
    use crate::vm::Vm;

    use super::*;

    #[test]
    fn lambda_compiles_to_bytecode() {
        let mut env = Vm::with_builtins().build_env();
        let instructions = Compiler::new("", &mut env)
            .compile_and_finalize(&Ast::from_sexp_str("(lambda (n) (+ n 1))").unwrap()[0])
            .unwrap()
            .bytecode
            .into_iter()
            .next()
            .unwrap();
        let bytecode = match instructions {
            Instruction::PushVal(Val::ByteCodeProc(proc)) => proc,
            v => panic!("Expected PushVal(ByteCodeProc) but found {v:?}"),
        };
        assert!(
            matches!(
                bytecode.bytecode.as_slice(),
                [
                    Instruction::PushVal(Val::NativeProc(_)),
                    Instruction::GetArg(0),
                    Instruction::PushVal(Val::Number(Number::Int(1))),
                    Instruction::Eval(3),
                ]
            ),
            "Found {:?}",
            bytecode.bytecode
        );
    }
}
