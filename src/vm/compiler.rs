use std::collections::HashMap;

use anyhow::{bail, Result};

use crate::parser::{
    ast::{Ast, AstLeaf},
    token::Token,
};

use super::{
    bytecode::OpCode,
    environment::Environment,
    types::{GenericProcedure, Number, Procedure, Symbol, Val},
    Vm,
};

/// A procedure that can be evaluated on an environment.
pub struct ByteCodeProc {
    /// The name of the procedure.
    name: String,
    /// The number of arguments to the procedure.
    arg_count: usize,
    /// The bytecode to run.
    bytecode: Vec<OpCode>,
}

impl GenericProcedure for ByteCodeProc {
    fn name(&self) -> &str {
        self.name.as_str()
    }

    fn eval(&self, env: &mut Environment) -> Result<Val> {
        let expected_args = self.arg_count;
        let stack_base = env.local.stack_base;
        let actual_args = env.stack.len() - stack_base;
        let initial_stack_len = env.stack.len();
        if expected_args != actual_args {
            bail!("procedure expected {expected_args} args but got {actual_args}");
        }
        env.eval_bytecode(&self.bytecode)?;
        let val = if env.stack.len() > initial_stack_len {
            env.stack.pop().unwrap()
        } else {
            Val::Void
        };
        Ok(val)
    }
}

impl ByteCodeProc {
    /// Iterate through all the instructions of the current procedure.
    pub fn instructions(&self) -> &[OpCode] {
        self.bytecode.as_slice()
    }
}

/// Compiles Asts into `ByteCodeProc` objects.
pub struct Compiler {
    symbol_to_idx: HashMap<String, usize>,
    opcodes: Vec<OpCode>,
}

impl Compiler {
    /// Create a new compiler.
    pub fn new() -> Compiler {
        Compiler::with_args(std::iter::empty())
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
    pub fn compile_and_finalize(self, name: String, ast: &Ast) -> Result<ByteCodeProc> {
        let mut c = self;
        c.compile(ast)?;
        Ok(c.finalize(name))
    }

    /// Finalize the current compilation and return the result as a callable `ByteCodeProc`.
    fn finalize(self, name: String) -> ByteCodeProc {
        let mut bytecode = self.opcodes;
        if bytecode.is_empty() {
            bytecode.push(OpCode::PushVal(Val::Void));
        }
        bytecode.shrink_to_fit();
        let arg_count = self
            .symbol_to_idx
            .values()
            .map(|idx| idx + 1)
            .max()
            .unwrap_or(0);
        ByteCodeProc {
            name,
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
                        self.opcodes.push(OpCode::Eval(children.len()));
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
                    self.opcodes.push(OpCode::Eval(vals.len()));
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
                Some(idx) => self.opcodes.push(OpCode::GetArg(*idx)),
                None => {
                    let val = Vm::singleton()
                        .globals
                        .lock()
                        .unwrap()
                        .get(x.as_str())
                        .cloned();
                    match val {
                        Some(v) => self.opcodes.push(OpCode::PushVal(v)),
                        None => self.opcodes.push(OpCode::GetSym(x.as_str().into())),
                    }
                }
            },
            AstLeaf::Symbol(x) => self
                .opcodes
                .push(OpCode::PushVal(Symbol::from(x.clone()).into())),
            AstLeaf::String(x) => self.opcodes.push(OpCode::PushVal(x.clone().into())),
            AstLeaf::Float(x) => self.opcodes.push(OpCode::PushVal(Number::Float(*x).into())),
            AstLeaf::Int(x) => self.opcodes.push(OpCode::PushVal(Number::Int(*x).into())),
            AstLeaf::Bool(x) => self.opcodes.push(OpCode::PushVal((*x).into())),
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
                lambda_compiler.opcodes.shrink_to_fit();
                let bytecode_proc = Procedure::new(ByteCodeProc {
                    name: "_".to_string(),
                    arg_count,
                    bytecode: lambda_compiler.opcodes,
                });
                self.opcodes.push(OpCode::PushVal(Val::Proc(bytecode_proc)));
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
                self.opcodes.extend([
                    OpCode::GetSym("%define-sym".into()),
                    OpCode::PushVal(sym.into()),
                ]);
                self.compile(expr)?;
                self.opcodes.push(OpCode::Eval(3));
            }
            _ => bail!("define requires 2 args but found {}", args.len()),
        };
        Ok(())
    }

    fn if_to_bytecode(&mut self, args: &[Ast]) -> Result<()> {
        match args {
            [pred, t_val, maybe_f_val @ ..] => {
                self.compile(pred)?;
                let t_bytecode = Compiler::with_symbols(self.symbol_to_idx.clone()).compile_and_finalize("".to_string(), t_val)?.bytecode;
                let f_bytecode = match maybe_f_val {
                    [] => vec![OpCode::PushVal(Val::Void)],
                    [f_val] => Compiler::with_symbols(self.symbol_to_idx.clone()).compile_and_finalize("".to_string(), f_val)?.bytecode,
                    rest => bail!("expected single false expression but found {}", rest.len()),
                };
                self.opcodes.push(OpCode::JumpIf(f_bytecode.len() + 1));
                self.opcodes.extend(f_bytecode);
                self.opcodes.push(OpCode::Jump(t_bytecode.len()));
                self.opcodes.extend(t_bytecode);
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
