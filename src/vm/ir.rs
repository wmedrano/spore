use std::{collections::HashMap, rc::Rc};

use anyhow::{anyhow, bail, Result};

use crate::parser::ast::{Ast, AstLeaf};

use super::types::{instruction::Instruction, proc::bytecode::ByteCodeProc, symbol::Symbol, Val};

#[derive(Debug, PartialEq)]
pub struct CodeBlock {
    pub name: Option<String>,
    pub arg_to_idx: HashMap<String, usize>,
    pub instructions: Vec<IrInstruction>,
}

#[derive(Debug, PartialEq)]
pub enum IrInstruction {
    Constant(Val),
    Procedure(CodeBlock),
    DerefIdentifier(String),
    CallProc {
        proc: Box<IrInstruction>,
        args: Vec<IrInstruction>,
    },
    If {
        pred: Box<IrInstruction>,
        true_expr: Box<IrInstruction>,
        false_expr: Option<Box<IrInstruction>>,
    },
    SetGlobal {
        sym: Symbol,
        value: Box<IrInstruction>,
    },
}

impl CodeBlock {
    pub fn with_ast<'a>(
        name: Option<String>,
        arg_to_idx: HashMap<String, usize>,
        asts: impl Iterator<Item = &'a Ast>,
    ) -> Result<CodeBlock> {
        let mut cb = CodeBlock::new(name, arg_to_idx);
        for ast in asts {
            cb.add_ast(ast)?;
        }
        Ok(cb)
    }

    pub fn new(name: Option<String>, arg_to_idx: HashMap<String, usize>) -> CodeBlock {
        CodeBlock {
            name,
            arg_to_idx,
            instructions: Vec::new(),
        }
    }

    pub fn add_ast(&mut self, ast: &Ast) -> Result<()> {
        self.instructions.push(self.make_instruction(ast)?);
        Ok(())
    }

    fn make_instruction(&self, ast: &Ast) -> Result<IrInstruction> {
        let res = match ast {
            Ast::Leaf(l) => match &l.item {
                AstLeaf::If => bail!(
                    "unexpected keyword if, did you mean (if <pred> <true-expr> <false-expr>)?"
                ),
                AstLeaf::Lambda => {
                    bail!(
                        "unexpected keyword lambda, did you mean (lambda (<args>...) <exprs>...)?"
                    )
                }
                AstLeaf::Define => {
                    bail!("unexpected keyword define, did you mean (define <symbol> <value-expr>)?")
                }
                AstLeaf::Identifier(ident) => IrInstruction::DerefIdentifier(ident.clone()),
                AstLeaf::Symbol(sym) => IrInstruction::Constant(Symbol::from(sym.clone()).into()),
                AstLeaf::String(s) => IrInstruction::Constant(Val::String(Rc::new(s.clone()))),
                AstLeaf::Float(f) => IrInstruction::Constant(Val::Float(*f)),
                AstLeaf::Int(i) => IrInstruction::Constant(Val::Int(*i)),
                AstLeaf::Bool(b) => IrInstruction::Constant(Val::Bool(*b)),
                AstLeaf::Comment(_) => IrInstruction::Constant(Val::Void),
                AstLeaf::CommentDatum => IrInstruction::Constant(Val::Void),
            },
            Ast::Tree(tree) => {
                let mut children = tree.iter();
                let first = match children.next() {
                    Some(c) => c,
                    None => bail!("Empty expression () is not valid"),
                };
                match first {
                    Ast::Leaf(l) => {
                        match &l.item {
                            AstLeaf::If => {
                                let pred = children.next().ok_or_else(|| anyhow!("if expected <pred>, <true-expr>, and optionally <false-expr>."))?;
                                let true_expr = children.next().ok_or_else(|| {
                                    anyhow!("if expected <true-expr>, and optionally <false-expr>.")
                                })?;
                                let false_expr = children.next();
                                if children.next().is_some() {
                                    bail!("if expression had too many args but expected only <pred>, <true-expr>, and optionally <false-expr>.");
                                }
                                self.make_if(pred, true_expr, false_expr)?
                            }
                            AstLeaf::Lambda => {
                                let args = match children.next().and_then(Ast::as_identifier_list) {
                                    Some(args) => args,
                                    None => bail!("lambda expected form (lambda (<args>...) <exprs>...) but (<args>...) was malformed"),
                                };
                                self.make_lambda(None, &args, children)?
                            }
                            AstLeaf::Define => {
                                let sym_expr = children.next();
                                if let Some(sym) = sym_expr.and_then(Ast::as_identifier) {
                                    let expr = children.next().ok_or_else(|| {
                                        anyhow!("define expected form (define <identifier> <expr>)")
                                    })?;
                                    if children.next().is_some() {
                                        bail!("define expected form (define <identifier> <expr>)")
                                    }
                                    self.make_define(Symbol::from(sym), expr)?
                                } else if let Some(syms) =
                                    sym_expr.and_then(Ast::as_identifier_list)
                                {
                                    match &syms[..] {
                                        [] => bail!("define form expected (<sym> <args>...) but found empty expression"),
                                        [name, args @ ..] => {
                                            let lambda = self.make_lambda(Some(name.to_string()), args, children)?;
                                            self.make_define_with_ir(Symbol::from(*name), lambda)
                                        },
                                    }
                                } else {
                                    bail!("define expected the form (define <sym> <expr>) or (define (<sym> <args>...) <exprs>...)");
                                }
                            }
                            AstLeaf::Identifier(ident) => self.make_proc_call(
                                IrInstruction::DerefIdentifier(ident.clone()),
                                children,
                            )?,
                            AstLeaf::Symbol(_)
                            | AstLeaf::String(_)
                            | AstLeaf::Float(_)
                            | AstLeaf::Int(_)
                            | AstLeaf::Bool(_) => {
                                bail!("atom is not callable")
                            }
                            AstLeaf::Comment(_) | AstLeaf::CommentDatum => {
                                unreachable!("AST iterator produced unexpected comment")
                            }
                        }
                    }
                    proc_ast @ Ast::Tree(_) => {
                        self.make_proc_call(self.make_instruction(proc_ast)?, children)?
                    }
                }
            }
        };
        Ok(res)
    }

    fn make_if(
        &self,
        pred: &Ast,
        true_expr: &Ast,
        false_expr: Option<&Ast>,
    ) -> Result<IrInstruction> {
        let pred = Box::new(self.make_instruction(pred)?);
        let true_expr = Box::new(self.make_instruction(true_expr)?);
        match false_expr {
            None => Ok(IrInstruction::If {
                pred,
                true_expr,
                false_expr: None,
            }),
            Some(expr) => Ok(IrInstruction::If {
                pred,
                true_expr,
                false_expr: Some(Box::new(self.make_instruction(expr)?)),
            }),
        }
    }

    fn make_define(&self, sym: Symbol, expr: &Ast) -> Result<IrInstruction> {
        let value = self.make_instruction(expr)?;
        Ok(self.make_define_with_ir(sym, value))
    }

    fn make_define_with_ir(&self, sym: Symbol, value: IrInstruction) -> IrInstruction {
        IrInstruction::SetGlobal {
            sym,
            value: Box::new(value),
        }
    }

    fn make_lambda<'a>(
        &self,
        name: Option<String>,
        args: &[&str],
        exprs: impl Iterator<Item = &'a Ast>,
    ) -> Result<IrInstruction> {
        let arg_to_idx = HashMap::from_iter(
            args.iter()
                .enumerate()
                .map(|(idx, sym)| (sym.to_string(), idx)),
        );
        let body = CodeBlock::with_ast(name, arg_to_idx, exprs)?;
        if body.instructions.is_empty() {
            bail!("lambda definition requires at least one <expr>.");
        }
        Ok(IrInstruction::Procedure(body))
    }

    fn make_proc_call<'a>(
        &self,
        proc: IrInstruction,
        arg_asts: impl Iterator<Item = &'a Ast>,
    ) -> Result<IrInstruction> {
        let mut args = Vec::new();
        for ast in arg_asts {
            args.push(self.make_instruction(ast)?);
        }
        Ok(IrInstruction::CallProc {
            proc: Box::new(proc),
            args,
        })
    }

    pub fn to_bytecode(&self) -> Result<ByteCodeProc> {
        Ok(ByteCodeProc {
            name: self.name.clone().unwrap_or_else(|| "_".to_string()),
            arg_count: self.arg_to_idx.len(),
            bytecode: self.to_bytecode_instructions(self.instructions.iter())?,
        })
    }

    pub fn to_bytecode_instructions<'a>(
        &self,
        irs: impl Iterator<Item = &'a IrInstruction>,
    ) -> Result<Vec<Instruction>> {
        let mut res: Vec<Instruction> = Vec::new();
        for ir in irs {
            match ir {
                IrInstruction::Constant(val) => res.push(Instruction::PushVal(val.clone())),
                IrInstruction::Procedure(codeblock) => {
                    let bytecode = codeblock.to_bytecode()?;
                    res.push(Instruction::PushVal(Val::ByteCodeProc(Rc::new(bytecode))));
                }
                IrInstruction::DerefIdentifier(ident) => {
                    match self.arg_to_idx.get(ident.as_str()) {
                        Some(idx) => res.push(Instruction::GetArg(*idx)),
                        None => res.push(Instruction::GetVal(Symbol::from(ident.as_str()))),
                    }
                }
                IrInstruction::CallProc { proc, args } => {
                    res.extend(self.to_bytecode_instructions(std::iter::once(proc.as_ref()))?);
                    for arg in args {
                        res.extend(self.to_bytecode_instructions(std::iter::once(arg))?);
                    }
                    res.push(Instruction::Eval(args.len() + 1));
                }
                IrInstruction::If {
                    pred,
                    true_expr,
                    false_expr,
                } => {
                    res.extend(self.to_bytecode_instructions(std::iter::once(pred.as_ref()))?);
                    let true_bytecode =
                        self.to_bytecode_instructions(std::iter::once(true_expr.as_ref()))?;
                    let false_bytecode = match false_expr {
                        Some(ir) => self.to_bytecode_instructions(std::iter::once(ir.as_ref()))?,
                        None => vec![Instruction::PushVal(Val::Void)],
                    };
                    res.push(Instruction::JumpIf(false_bytecode.len() + 1));
                    res.extend(false_bytecode);
                    res.push(Instruction::Jump(true_bytecode.len()));
                    res.extend(true_bytecode);
                }
                IrInstruction::SetGlobal { sym, value } => {
                    res.extend(self.to_bytecode_instructions(std::iter::once(value.as_ref()))?);
                    res.push(Instruction::SetVal(sym.clone()));
                }
            };
        }
        Ok(res)
    }
}