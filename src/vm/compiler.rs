use std::{collections::HashMap, sync::Arc};

use anyhow::{bail, Result};

use crate::parser::{
    ast::{Ast, AstLeaf},
    token::Token,
};

use super::{
    types::{instruction::Instruction, proc::Procedure, symbol::Symbol, Number, Val},
    Vm,
};

/// A procedure that can be evaluated on an environment.
pub struct ByteCodeProc {
    /// The number of arguments to the procedure.
    pub arg_count: usize,
    /// The bytecode to run.
    bytecode: Arc<[Instruction]>,
}

impl ByteCodeProc {
    /// Get all the instructions in the procedure.
    pub fn instructions(&self) -> Arc<[Instruction]> {
        self.bytecode.clone()
    }

    pub fn iter_instructions(&self) -> impl Iterator<Item = &Instruction> {
        self.bytecode.iter()
    }
}

/// Compiles Asts into `ByteCodeProc` objects.
#[derive(Default)]
pub struct Compiler {
    symbol_to_idx: HashMap<String, usize>,
    opcodes: Vec<Instruction>,
}

impl Compiler {
    /// Create a new compiler.
    pub fn new() -> Compiler {
        Compiler::default()
    }

    /// Create a new compiler with the given arguments at the base of the stack.
    pub fn with_args(args: impl Iterator<Item = String>) -> Compiler {
        let symbol_to_idx = args.enumerate().map(|(idx, sym)| (sym, idx)).collect();
        Compiler::with_symbols(symbol_to_idx)
    }

    /// Create a new compiler where the base of the stack contains the values from `symbol_to_idx`.
    pub fn with_symbols(symbol_to_idx: HashMap<String, usize>) -> Compiler {
        Compiler {
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
        let arg_count = self
            .symbol_to_idx
            .values()
            .map(|idx| idx + 1)
            .max()
            .unwrap_or(0);
        ByteCodeProc {
            arg_count,
            bytecode: Arc::from(bytecode.as_slice()),
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
                    AstLeaf::Def => self.def_to_bytecode(args)?,
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
            AstLeaf::Def => bail!("empty def is not a valid expression"),
            AstLeaf::Identifier(x) => match self.symbol_to_idx.get(x) {
                Some(idx) => self.opcodes.push(Instruction::GetArg(*idx)),
                None => {
                    let val = Vm::singleton()
                        .globals
                        .lock()
                        .unwrap()
                        .get(x.as_str())
                        .cloned();
                    match val {
                        Some(v) => self.opcodes.push(Instruction::PushVal(v)),
                        None => self.opcodes.push(Instruction::GetSym(x.as_str().into())),
                    }
                }
            },
            AstLeaf::Symbol(x) => self
                .opcodes
                .push(Instruction::PushVal(Symbol::from(x.clone()).into())),
            AstLeaf::String(x) => self.opcodes.push(Instruction::PushVal(x.clone().into())),
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
                let arg_count = symbol_to_idx.len();
                let mut lambda_compiler = Compiler {
                    symbol_to_idx,
                    opcodes: Vec::new(),
                };
                for expr in exprs {
                    lambda_compiler.compile(expr)?;
                }
                let bytecode_proc = Procedure::with_bytecode(ByteCodeProc {
                    arg_count,
                    bytecode: Arc::from(lambda_compiler.opcodes),
                });
                self.opcodes
                    .push(Instruction::PushVal(Val::Proc(bytecode_proc)));
            }
        };
        Ok(())
    }

    fn def_to_bytecode(&mut self, args: &[Ast]) -> Result<()> {
        match args {
            [sym, expr] => {
                let sym = match sym {
                    Ast::Leaf(Token {
                        item: AstLeaf::Identifier(ident),
                        ..
                    }) => Symbol::from(ident.as_str()),
                    ast => bail!("def must be bound to an identifier but found {:?}", ast),
                };
                self.opcodes.extend([
                    Instruction::GetSym("%define-sym".into()),
                    Instruction::PushVal(sym.into()),
                ]);
                self.compile(expr)?;
                self.opcodes.push(Instruction::Eval(3));
            }
            _ => bail!("def requires 2 args but found {}", args.len()),
        };
        Ok(())
    }

    fn if_to_bytecode(&mut self, args: &[Ast]) -> Result<()> {
        match args {
            [pred, t_val, maybe_f_val @ ..] => {
                self.compile(pred)?;
                let t_bytecode = Compiler::with_symbols(self.symbol_to_idx.clone()).compile_and_finalize(t_val)?.bytecode;
                let f_bytecode = match maybe_f_val {
                    [] => vec![Instruction::PushVal(Val::Void)],
                    [f_val] => Compiler::with_symbols(self.symbol_to_idx.clone()).compile_and_finalize(f_val)?.bytecode.to_vec(),
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
