use std::collections::HashMap;

use anyhow::{bail, Result};

use crate::parser::{
    ast::{Ast, AstLeaf},
    token::Token,
};

use super::{
    environment::Environment,
    types::{instruction::Instruction, proc::bytecode::ByteCodeProc, symbol::Symbol, Val},
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
    pub fn compile(self, ast: Ast) -> Result<ByteCodeProc> {
        self.compile_pruned(&ast)
    }

    fn compile_pruned(self, ast: &Ast) -> Result<ByteCodeProc> {
        let mut c = self;
        c.compile_impl(ast)?;
        Ok(c.finalize())
    }

    fn finalize(self) -> ByteCodeProc {
        let mut bytecode = self.opcodes;
        if bytecode.is_empty() {
            bytecode.push(Instruction::PushVal(Val::Void));
        }
        for bc in bytecode.iter_mut() {
            if let Instruction::GetVal(sym) = bc {
                if let Some(val) = self.env.get_global(sym) {
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

    fn compile_impl(&mut self, ast: &Ast) -> Result<()> {
        match ast {
            Ast::Leaf(l) => self.leaf_to_bytecode(&l.item)?,
            Ast::Tree(children) => {
                let mut children = iter_no_comments(&children);
                let first_child = match children.next() {
                    Some(child) => child,
                    None => bail!("found empty expression"),
                };
                match first_child {
                    Ast::Leaf(leaf) => match leaf.item {
                        AstLeaf::If => self.if_to_bytecode(children)?,
                        AstLeaf::Lambda => self.lambda_to_bytecode(children)?,
                        AstLeaf::Define => self.define_to_bytecode(children)?,
                        AstLeaf::Identifier(_) => {
                            self.compile_impl(first_child)?;
                            let mut eval_n = 1;
                            for c in children {
                                eval_n += 1;
                                self.compile_impl(c)?;
                            }
                            self.opcodes.push(Instruction::Eval(eval_n));
                        }
                        AstLeaf::Symbol(_)
                        | AstLeaf::String(_)
                        | AstLeaf::Float(_)
                        | AstLeaf::Int(_)
                        | AstLeaf::Bool(_) => bail!("cannot evaluate object {:?}", first_child),
                        AstLeaf::Comment(_) => unreachable!(),
                    },
                    Ast::Tree(_) => {
                        self.compile_impl(first_child)?;
                        let mut eval_n = 1;
                        for c in children {
                            eval_n += 1;
                            self.compile_impl(c)?;
                        }
                        self.opcodes.push(Instruction::Eval(eval_n));
                    }
                };
            }
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
            AstLeaf::Bool(x) => self.opcodes.push(Instruction::PushVal((*x).into())),
            AstLeaf::Int(x) => self.opcodes.push(Instruction::PushVal((*x).into())),
            AstLeaf::Float(x) => self.opcodes.push(Instruction::PushVal((*x).into())),
            AstLeaf::String(x) => self.opcodes.push(Instruction::PushVal(x.clone().into())),
            AstLeaf::Symbol(x) => self
                .opcodes
                .push(Instruction::PushVal(Symbol::from(x.clone()).into())),
            AstLeaf::Comment(_) => (),
        }
        Ok(())
    }

    fn lambda_to_bytecode<'b>(&mut self, expr: impl Iterator<Item = &'b Ast>) -> Result<()> {
        let mut expr = expr;
        let lambda_args = match expr.next() {
            Some(a) => a,
            None => bail!("found (lambda) but lambda must have form (lambda (..args) exprs)"),
        };
        let args = match lambda_args {
            Ast::Leaf(_) => bail!("first argument to lambda must be a list of arguments"),
            Ast::Tree(args) => args,
        };
        self.lambda_to_bytecode_impl(iter_no_comments(&args), expr)?;
        Ok(())
    }

    fn lambda_to_bytecode_impl<'b>(
        &mut self,
        args: impl Iterator<Item = &'b Ast>,
        exprs: impl Iterator<Item = &'b Ast>,
    ) -> Result<()> {
        let symbol_to_idx = symbol_to_idx(args)?;
        let mut lambda_compiler = Compiler {
            env: self.env,
            name: self.name.clone(),
            symbol_to_idx,
            opcodes: Vec::new(),
        };
        for expr in exprs {
            lambda_compiler.compile_impl(expr)?;
        }
        let bytecode = lambda_compiler.finalize();
        self.opcodes.push(Instruction::PushVal(bytecode.into()));
        Ok(())
    }

    fn define_to_bytecode<'b>(&mut self, args: impl Iterator<Item = &'b Ast>) -> Result<()> {
        let mut args = args;
        match args.next() {
            None => bail!("define requires 2 args but found none"),
            Some(Ast::Leaf(Token { item, .. })) => {
                let symbol = match item {
                    AstLeaf::Identifier(ident) => Symbol::from(ident.as_str()),
                    _ => bail!("define expected identifier but found {:?}", item),
                };
                let expr = match args.next() {
                    None => bail!("define requires 2 args but found only {symbol}"),
                    Some(expr) => expr,
                };
                let rest_count = args.count();
                if rest_count > 0 {
                    bail!("define requires 2 args but found {}", rest_count + 2);
                }
                self.define_with_symbol_to_bytecode(symbol, expr)?;
            }
            Some(Ast::Tree(name_and_args)) => match name_and_args.as_slice() {
                [Ast::Leaf(Token {
                    item: AstLeaf::Identifier(ident),
                    ..
                }), proc_args @ ..] => {
                    let symbol = Symbol::from(ident.as_str());
                    self.define_proc_to_bytecode(symbol, iter_no_comments(proc_args), args)?
                }
                _ => bail!("bad"),
            },
        };
        Ok(())
    }

    fn define_with_symbol_to_bytecode(&mut self, symbol: Symbol, expr: &Ast) -> Result<()> {
        let mut tmp_name = symbol.as_str().to_string();
        std::mem::swap(&mut tmp_name, &mut self.name);
        self.compile_impl(expr)?;
        std::mem::swap(&mut tmp_name, &mut self.name);
        self.opcodes.push(Instruction::SetVal(symbol));
        Ok(())
    }

    fn define_proc_to_bytecode<'b>(
        &mut self,
        symbol: Symbol,
        args: impl Iterator<Item = &'b Ast>,
        exprs: impl Iterator<Item = &'b Ast>,
    ) -> Result<()> {
        let mut tmp_name = symbol.as_str().to_string();
        std::mem::swap(&mut tmp_name, &mut self.name);
        self.lambda_to_bytecode_impl(args, exprs)?;
        std::mem::swap(&mut tmp_name, &mut self.name);
        self.opcodes.push(Instruction::SetVal(symbol));
        Ok(())
    }

    fn if_to_bytecode<'b>(&mut self, args: impl Iterator<Item = &'b Ast>) -> Result<()> {
        let mut args = args;
        let pred = args.next().unwrap();
        let t_val = args.next().unwrap();
        let maybe_f_val = args.next();
        if args.next().is_some() {
            bail!("expected single false expression");
        }
        self.compile_impl(pred)?;
        let t_bytecode = Compiler::with_symbols("", self.env, self.symbol_to_idx.clone())
            .compile_pruned(t_val)?
            .bytecode;
        let f_bytecode = match maybe_f_val {
            None => vec![Instruction::PushVal(Val::Void)],
            Some(f_val) => Compiler::with_symbols("", self.env, self.symbol_to_idx.clone())
                .compile_pruned(f_val)?
                .bytecode
                .to_vec(),
        };
        self.opcodes.push(Instruction::JumpIf(f_bytecode.len() + 1));
        self.opcodes.extend(f_bytecode);
        self.opcodes.push(Instruction::Jump(t_bytecode.len()));
        self.opcodes.extend(t_bytecode.iter().cloned());
        Ok(())
    }
}

fn symbol_to_idx<'a>(list: impl Iterator<Item = &'a Ast>) -> Result<HashMap<String, usize>> {
    let mut res = HashMap::new();
    for (idx, element) in list.enumerate() {
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

/// Iterate over the asts, skipping any comment nodes.
fn iter_no_comments<'a>(asts: &'a [Ast]) -> impl Iterator<Item = &'a Ast> {
    asts.iter().filter(|ast| {
        !matches!(
            ast,
            Ast::Leaf(Token {
                item: AstLeaf::Comment(_),
                ..
            })
        )
    })
}

#[cfg(test)]
mod tests {
    use crate::vm::Vm;

    use super::*;

    #[test]
    fn lambda_compiles_to_bytecode() {
        let mut env = Vm::new().build_env();
        let ast = Ast::from_sexp_str("(lambda (n) (+ n 1))").unwrap();
        let instructions = Compiler::new("", &mut env)
            .compile(ast[0].clone())
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
                    Instruction::PushVal(Val::Int(1)),
                    Instruction::Eval(3),
                ]
            ),
            "Found {:?}",
            bytecode.bytecode
        );
    }
}
