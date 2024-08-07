use std::{collections::HashMap, path::PathBuf, rc::Rc, str::FromStr};

use anyhow::{anyhow, bail, ensure, Result};
use smol_str::SmolStr;

use crate::parser::{
    ast::{Ast, AstLeaf},
    token::Token,
};

use super::{
    module::ModuleSource,
    types::{
        instruction::{Instruction, ValRef},
        proc::bytecode::ByteCodeProc,
        symbol::Symbol,
        Val,
    },
};

/// Contains a set of high level instructions to execute. Can be converted to lower level
/// instructions.
#[derive(Debug, PartialEq)]
pub struct CodeBlock {
    /// The name of the code block or `None` if it has no name.
    pub name: Option<SmolStr>,
    /// A map from a symbol name to its index at the frame stack.
    pub arg_to_idx: HashMap<SmolStr, usize>,
    /// The IR instructions for the codeblock.
    pub instructions: Vec<IrInstruction>,
}

#[derive(Debug, PartialEq)]
pub enum IrInstruction {
    /// Push a constant value to the stack.
    PushConst(Val),
    /// Push a procedure value to the stack.
    PushProc(CodeBlock),
    /// Dereference an identifier and push it to the stack.
    DerefIdentifier { symbol: String },
    /// Call a procedure with the given arguments.
    CallProc {
        proc: Box<IrInstruction>,
        args: Vec<IrInstruction>,
    },
    /// Execute either `true_expr` or `false_expr` depending on the value of `pred`.
    If {
        pred: Box<IrInstruction>,
        true_expr: Box<IrInstruction>,
        false_expr: Option<Box<IrInstruction>>,
    },
    /// Set `sym` to `value` globally.
    Define {
        symbol: Symbol,
        value: Box<IrInstruction>,
    },
    /// Ensure that a file has been imported.
    Import { filepath: PathBuf },
    /// Execute a bytecode instruction.
    Raw(Instruction),
}

#[derive(Debug, Default, Clone)]
pub struct CodeBlockArgs {
    /// The name of the codeblock. This is used to populate the name of the produced bytecode
    /// procedure.
    pub name: Option<SmolStr>,
    /// A map from argument name to the index it should appear on the stack frame.
    pub arg_to_idx: HashMap<SmolStr, usize>,
    /// If the code block defines a subexpression. Certain features (such as import and define) are
    /// not available as a subexpression.
    pub is_subexpression: bool,
}

impl CodeBlock {
    /// Create a new code block with `asts`.
    pub fn with_ast<'a>(
        args: CodeBlockArgs,
        asts: impl Iterator<Item = &'a Ast>,
    ) -> Result<CodeBlock> {
        let is_subexpression = args.is_subexpression;
        let mut cb = CodeBlock {
            name: args.name,
            arg_to_idx: args.arg_to_idx,
            instructions: Vec::new(),
        };
        for ast in asts {
            let instructions = cb.make_instruction(ast, is_subexpression)?;
            cb.instructions.push(instructions);
        }
        Ok(cb)
    }

    /// Convert the codeblock into a procedure that defines a module.
    pub fn to_module_definition(&self, module: ModuleSource) -> Result<ByteCodeProc> {
        self.to_proc_impl(module, true)
    }

    /// Convert the code block into bytecode that is attached to the given module.
    pub fn to_proc(&self, module: ModuleSource) -> Result<ByteCodeProc> {
        self.to_proc_impl(module, false)
    }

    fn to_proc_impl(
        &self,
        module: ModuleSource,
        is_module_definition: bool,
    ) -> Result<ByteCodeProc> {
        let bytecode = self.to_bytecode_instructions(&module, self.instructions.iter())?;
        Ok(ByteCodeProc {
            name: self.name.clone().unwrap_or(SmolStr::new_static("_")),
            arg_count: self.arg_to_idx.len(),
            bytecode,
            module,
            is_module_definition,
        })
    }

    /// Make a new `IrInstruction`. If `is_subexpression` is `true`, then some instructions such as
    /// `define` and `import` will not be available.
    fn make_instruction(&self, ast: &Ast, is_subexpression: bool) -> Result<IrInstruction> {
        let res = match ast {
            Ast::Leaf(l) => match &l.item {
                AstLeaf::If => bail!(
                    "Unexpected keyword if, did you mean (if <pred> <true-expr> <false-expr>)?"
                ),
                AstLeaf::Import => {
                    bail!("Unexpected keyword import, did you mean (import \"<file>\").")
                }
                AstLeaf::Lambda => {
                    bail!(
                        "Unexpected keyword lambda, did you mean (lambda (<args>...) <exprs>...)?"
                    )
                }
                AstLeaf::Define => {
                    bail!("Unexpected keyword define, did you mean (define <symbol> <value-expr>)?")
                }
                AstLeaf::Identifier(ident) => IrInstruction::DerefIdentifier {
                    symbol: ident.clone(),
                },
                AstLeaf::Symbol(sym) => IrInstruction::PushConst(Symbol::from(sym.as_str()).into()),
                AstLeaf::String(s) => IrInstruction::PushConst(Val::String(Rc::new(s.clone()))),
                AstLeaf::Float(f) => IrInstruction::PushConst(Val::Float(*f)),
                AstLeaf::Int(i) => IrInstruction::PushConst(Val::Int(*i)),
                AstLeaf::Bool(b) => IrInstruction::PushConst(Val::Bool(*b)),
                AstLeaf::Comment(_) => IrInstruction::PushConst(Val::Void),
                AstLeaf::CommentDatum => IrInstruction::PushConst(Val::Void),
            },
            Ast::Tree(tree) => {
                let mut children = tree.iter();
                let first = match children.next() {
                    Some(c) => c,
                    None => bail!("Empty expression () is not valid"),
                };
                match first {
                    Ast::Leaf(l) => match &l.item {
                        AstLeaf::If => {
                            let pred = children.next().ok_or_else(|| {
                                anyhow!(
                                    "if expected <pred>, <true-expr>, and optionally <false-expr>."
                                )
                            })?;
                            let true_expr = children.next().ok_or_else(|| {
                                anyhow!("if expected <true-expr>, and optionally <false-expr>.")
                            })?;
                            let false_expr = children.next();
                            if children.next().is_some() {
                                bail!("if expression had too many args but expected only <pred>, <true-expr>, and optionally <false-expr>.");
                            }
                            self.make_if(pred, true_expr, false_expr)?
                        }
                        AstLeaf::Import => {
                            ensure!(
                                !is_subexpression,
                                "(import ...) not allowed as a subexpression."
                            );
                            let filepath = children.next().ok_or_else(|| {
                                anyhow!("expected expression of form (import \"filepath\").")
                            })?;
                            self.make_import(filepath)?
                        }
                        AstLeaf::Lambda => {
                            let args = match children.next().and_then(Ast::as_identifier_list) {
                                    Some(args) => args,
                                    None => bail!("lambda expected form (lambda (<args>...) <exprs>...) but (<args>...) was malformed."),
                                };
                            self.make_lambda(None, &args, children)?
                        }
                        AstLeaf::Define => {
                            ensure!(
                                !is_subexpression,
                                "(define ...) not allowed as a subexpression."
                            );
                            let sym_expr = children.next();
                            if let Some(sym) = sym_expr.and_then(Ast::as_identifier) {
                                let expr = children.next().ok_or_else(|| {
                                    anyhow!("define expected form (define <identifier> <expr>).")
                                })?;
                                if children.next().is_some() {
                                    bail!("define expected form (define <identifier> <expr>).")
                                }
                                self.make_define(Symbol::from(sym), expr)?
                            } else if let Some(syms) = sym_expr.and_then(Ast::as_identifier_list) {
                                match &syms[..] {
                                        [] => bail!("define form expected (<sym> <args>...) but found empty expression."),
                                        [name, args @ ..] => {
                                            let lambda = self.make_lambda(Some(SmolStr::new(name)), args, children)?;
                                            self.make_define_with_ir(Symbol::from(*name), lambda)
                                        },
                                    }
                            } else {
                                bail!("define expected the form (define <sym> <expr>) or (define (<sym> <args>...) <exprs>...).");
                            }
                        }
                        AstLeaf::Identifier(ident) => self.make_proc_call(
                            IrInstruction::DerefIdentifier {
                                symbol: ident.clone(),
                            },
                            children,
                        )?,
                        AstLeaf::Symbol(_)
                        | AstLeaf::String(_)
                        | AstLeaf::Float(_)
                        | AstLeaf::Int(_)
                        | AstLeaf::Bool(_) => {
                            bail!(
                                "Expected value of type <proc>, {v} is not callable.",
                                v = first,
                            )
                        }
                        AstLeaf::Comment(_) | AstLeaf::CommentDatum => {
                            unreachable!("AST iterator produced unexpected comment.")
                        }
                    },
                    proc_ast @ Ast::Tree(_) => {
                        self.make_proc_call(self.make_instruction(proc_ast, true)?, children)?
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
        let pred = Box::new(self.make_instruction(pred, true)?);
        let true_expr = Box::new(self.make_instruction(true_expr, true)?);
        match false_expr {
            None => Ok(IrInstruction::If {
                pred,
                true_expr,
                false_expr: None,
            }),
            Some(expr) => Ok(IrInstruction::If {
                pred,
                true_expr,
                false_expr: Some(Box::new(self.make_instruction(expr, true)?)),
            }),
        }
    }

    fn make_import(&self, filepath: &Ast) -> Result<IrInstruction> {
        match filepath {
            Ast::Leaf(Token {
                item: AstLeaf::String(filepath),
                ..
            }) => {
                let filepath = PathBuf::from_str(filepath)?;
                Ok(IrInstruction::Import { filepath })
            }
            _ => bail!("Expected expression of form (import \"<filepath>\")."),
        }
    }

    fn make_define(&self, sym: Symbol, expr: &Ast) -> Result<IrInstruction> {
        let value = self.make_instruction(expr, true)?;
        Ok(self.make_define_with_ir(sym, value))
    }

    fn make_define_with_ir(&self, sym: Symbol, value: IrInstruction) -> IrInstruction {
        IrInstruction::Define {
            symbol: sym,
            value: Box::new(value),
        }
    }

    fn make_lambda<'a>(
        &self,
        name: Option<SmolStr>,
        args: &[&str],
        exprs: impl Iterator<Item = &'a Ast>,
    ) -> Result<IrInstruction> {
        let arg_to_idx = HashMap::from_iter(
            args.iter()
                .enumerate()
                .map(|(idx, sym)| (SmolStr::new(sym), idx)),
        );
        let body = CodeBlock::with_ast(
            CodeBlockArgs {
                name,
                arg_to_idx,
                is_subexpression: true,
            },
            exprs,
        )?;
        if body.instructions.is_empty() {
            bail!("lambda definition requires at least one <expr>.");
        }
        Ok(IrInstruction::PushProc(body))
    }

    fn make_proc_call<'a>(
        &self,
        proc: IrInstruction,
        arg_asts: impl Iterator<Item = &'a Ast>,
    ) -> Result<IrInstruction> {
        let mut args = Vec::new();
        for ast in arg_asts {
            let instructions = self.make_instruction(ast, true)?;
            args.push(instructions);
        }
        Ok(IrInstruction::CallProc {
            proc: Box::new(proc),
            args,
        })
    }
}

impl CodeBlock {
    fn to_bytecode_instructions<'a>(
        &self,
        default_module: &ModuleSource,
        irs: impl Iterator<Item = &'a IrInstruction>,
    ) -> Result<Vec<Instruction>> {
        let mut res: Vec<Instruction> = Vec::new();
        for ir in irs {
            match ir {
                IrInstruction::PushConst(val) => res.push(Instruction::PushVal(val.clone())),
                IrInstruction::PushProc(codeblock) => {
                    let bytecode = codeblock.to_proc(default_module.clone())?;
                    res.push(Instruction::PushVal(Val::ByteCodeProc(Rc::new(bytecode))));
                }
                IrInstruction::DerefIdentifier { symbol } => {
                    match self.arg_to_idx.get(symbol.as_str()) {
                        Some(idx) => res.push(Instruction::GetArg(*idx)),
                        None => {
                            let (module_ident, symbol) = split_identifier(symbol.as_str());
                            let val_ref = Box::new(ValRef {
                                module: default_module.clone(),
                                sub_module: module_ident.map(SmolStr::new),
                                symbol: SmolStr::new(symbol),
                            });
                            res.push(Instruction::GetVal(val_ref))
                        }
                    }
                }
                IrInstruction::CallProc { proc, args } => {
                    res.extend(self.to_bytecode_instructions(
                        default_module,
                        std::iter::once(proc.as_ref()),
                    )?);
                    for arg in args {
                        res.extend(
                            self.to_bytecode_instructions(default_module, std::iter::once(arg))?,
                        );
                    }
                    res.push(Instruction::Eval(args.len() + 1));
                }
                IrInstruction::If {
                    pred,
                    true_expr,
                    false_expr,
                } => {
                    res.extend(self.to_bytecode_instructions(
                        default_module,
                        std::iter::once(pred.as_ref()),
                    )?);
                    let true_bytecode = self.to_bytecode_instructions(
                        default_module,
                        std::iter::once(true_expr.as_ref()),
                    )?;
                    let false_bytecode = match false_expr {
                        Some(ir) => self.to_bytecode_instructions(
                            default_module,
                            std::iter::once(ir.as_ref()),
                        )?,
                        None => vec![Instruction::PushVal(Val::Void)],
                    };
                    res.push(Instruction::JumpIf(false_bytecode.len() + 1));
                    res.extend(false_bytecode);
                    res.push(Instruction::Jump(true_bytecode.len()));
                    res.extend(true_bytecode);
                }
                IrInstruction::Define { symbol: sym, value } => {
                    res.extend(self.to_bytecode_instructions(
                        default_module,
                        std::iter::once(value.as_ref()),
                    )?);
                    res.push(Instruction::SetVal(Box::new(sym.clone())));
                }
                IrInstruction::Import { filepath } => {
                    res.push(Instruction::ImportModule(Box::new(filepath.clone())));
                }
                IrInstruction::Raw(instruction) => {
                    res.push(instruction.clone());
                }
            };
        }
        Ok(res)
    }
}

fn split_identifier(s: &str) -> (Option<String>, String) {
    match s.find('/') {
        Some(idx) if idx > 0 => (Some(s[0..idx].to_string()), s[idx + 1..].to_string()),
        _ => (None, s.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::vm::types::{instruction::Instruction, Val};

    use super::*;

    const MODULE: ModuleSource = ModuleSource::Virtual("test");

    fn compile_to_codeblock(s: &str) -> anyhow::Result<CodeBlock> {
        let asts = Ast::from_sexp_str(s).unwrap();
        CodeBlock::with_ast(CodeBlockArgs::default(), asts.iter())
    }

    fn compile_to_bytecode(s: &str) -> anyhow::Result<Vec<Instruction>> {
        let asts = Ast::from_sexp_str(s).unwrap();
        let codeblock = CodeBlock::with_ast(CodeBlockArgs::default(), asts.iter()).unwrap();
        Ok(codeblock.to_proc(MODULE)?.bytecode)
    }

    #[test]
    fn lambda_compiles_to_val_with_bytecode() {
        let instructions = compile_to_bytecode("(lambda (n) (+ n 1))")
            .unwrap()
            .into_iter()
            .next()
            .unwrap();
        let bytecode = match instructions {
            Instruction::PushVal(Val::ByteCodeProc(proc)) => proc.bytecode.clone(),
            v => panic!("Expected PushVal(ByteCodeProc) but found {v:?}"),
        };
        assert!(
            matches!(
                bytecode.as_slice(),
                [
                    Instruction::GetVal(_),
                    Instruction::GetArg(0),
                    Instruction::PushVal(Val::Int(1)),
                    Instruction::Eval(3),
                ]
            ),
            "Found {bytecode:?}",
        );
    }

    #[test]
    fn comment_next_datum_skips_datum() {
        let bytecode = compile_to_bytecode("(+ 1 #; \"this is skipped\" #;2 3)").unwrap();
        assert!(
            matches!(
                bytecode.as_slice(),
                [
                    Instruction::GetVal(_),
                    Instruction::PushVal(Val::Int(1)),
                    Instruction::PushVal(Val::Int(3)),
                    Instruction::Eval(3),
                ]
            ),
            "Found {bytecode:?}",
        );
    }

    #[test]
    fn define_not_allowed_in_subexpressions() {
        assert!(compile_to_codeblock("(define a 4)").is_ok());
        assert!(compile_to_codeblock("(do (define b 4))").is_err());
    }

    #[test]
    fn import_not_allowed_in_subexpressions() {
        assert!(compile_to_codeblock("(import \"my-file.spore\")").is_ok());
        assert!(compile_to_codeblock("(do (import \"my-file.spore\"))").is_err());
    }

    #[test]
    fn import_compiles_to_import_instruction() {
        let asts = Ast::from_sexp_str("(import \"my-file.spore\")").unwrap();
        let codeblock = CodeBlock::with_ast(CodeBlockArgs::default(), asts.iter()).unwrap();
        let proc = codeblock.to_module_definition(MODULE).unwrap();

        assert!(!codeblock.to_proc(MODULE).unwrap().is_module_definition);
        assert!(proc.is_module_definition);

        let bytecode = proc.bytecode;
        assert!(
            matches!(bytecode.as_slice(), [Instruction::ImportModule(p)] if p.as_ref() == Path::new("my-file.spore")),
            "Got: {bytecode:?}"
        );
    }

    #[test]
    fn slash_separated_identifier_splits_module_and_symbol() {
        let bytecode = compile_to_bytecode("sub_module/symbol").unwrap();
        assert!(
            matches!(
                bytecode.as_slice(),
                [Instruction::GetVal(val_ref)] if matches!(val_ref.as_ref(), ValRef{module: MODULE, sub_module: Some(sub_module), symbol } if sub_module == "sub_module" && symbol == "symbol")
            ),
            "Got: {bytecode:?}"
        );
    }

    #[test]
    fn slash_converts_to_division_symbol() {
        let bytecode = compile_to_bytecode("/").unwrap();
        assert!(
            matches!(
                bytecode.as_slice(),
                [Instruction::GetVal(val_ref)] if matches!(val_ref.as_ref(), ValRef{module: MODULE, sub_module: None, symbol } if symbol == "/")
            ),
            "Got: {bytecode:?}"
        );
    }
}
