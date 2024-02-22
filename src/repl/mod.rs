use std::sync::Arc;

use anyhow::{bail, Result};
use colored::Colorize;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;

use crate::parser::ast::{Ast, ParseAstError};
use crate::vm::compiler::Compiler;
use crate::vm::types::instruction::Instruction;
use crate::vm::types::proc::{ByteCodeIter, ByteCodeProc};
use crate::vm::types::symbol::Symbol;
use crate::vm::types::Val;
use crate::vm::Vm;

use self::command::{Command, MetaCommand};

pub mod command;

pub struct Repl {
    editor: DefaultEditor,
    repl_input: String,
    expression_count: usize,
}

impl Repl {
    pub fn new() -> Result<Repl> {
        let editor = DefaultEditor::new()?;
        Ok(Repl {
            editor,
            repl_input: String::new(),
            expression_count: 0,
        })
    }

    /// Run the REPL.
    pub fn run(&mut self) -> Result<()> {
        println!("{}", "Welcome to Spore!".cyan());
        println!("{}", "  https://github.com/wmedrano/spore".cyan());
        println!();
        loop {
            let readline = self.editor.readline(if self.repl_input.is_empty() {
                ">> "
            } else {
                ".. "
            });
            match readline {
                Ok(line) => {
                    self.repl_input += line.as_str();
                    if line_is_complete(&self.repl_input) {
                        self.editor.add_history_entry(self.repl_input.as_str())?;
                        if let Err(err) = self.eval_input() {
                            println!("{error}\n{err}", error = "Error:".to_string().red());
                        }
                    }
                }
                Err(ReadlineError::Eof | ReadlineError::Interrupted) => {
                    println!();
                    break;
                }
                Err(err) => {
                    println!("{}\n{:?}", "Error".red(), err);
                    break;
                }
            }
        }
        Ok(())
    }

    pub fn eval_input(&mut self) -> Result<()> {
        let s = std::mem::take(&mut self.repl_input);
        let cmd = Command::try_from(s.as_str())?;
        let asts = match Ast::from_sexp_str(s.as_str()) {
            Ok(ast) => ast,
            Err(err) => {
                bail!("{}", err.display_with_context(s.as_str()));
            }
        };
        match cmd.command {
            MetaCommand::None => eval_asts(asts, &mut self.expression_count),
            MetaCommand::Ast => analyze_ast(asts),
            MetaCommand::ByteCode => analyze_bytecode(asts),
        }
        Ok(())
    }
}

fn line_is_complete(s: &str) -> bool {
    !matches!(
        Ast::from_sexp_str(s),
        Err(ParseAstError::MissingClosingParen { .. })
    )
}

fn eval_asts(asts: Vec<Ast>, expr_count: &mut usize) {
    for ast in asts {
        match eval_ast(&ast) {
            Ok(Val::Void) => (),
            Ok(v) => {
                *expr_count += 1;
                let sym = Symbol::from(format!("${expr_count}"));
                let _ = Vm::singleton().register_global_value(sym.clone(), v.clone());
                println!("{} = {}", sym.as_str().to_string().cyan(), v);
            }
            Err(err) => println!("{}", err.to_string().red()),
        }
    }
}

fn eval_ast(ast: &Ast) -> Result<Val> {
    let bytecode = Compiler::new().compile_and_finalize(ast)?;
    Vm::singleton().env().eval_bytecode(bytecode.into())
}

fn analyze_bytecode(asts: Vec<Ast>) {
    for ast in asts {
        let proc = match Compiler::new().compile_and_finalize(&ast) {
            Ok(b) => b,
            Err(err) => {
                println!("{}", err.to_string().red());
                continue;
            }
        };
        let bytecode = maybe_expand_bytecode(proc);
        for (idx, bc) in bytecode.enumerate() {
            println!("  {:02} - {bc}", format!("{:02}", idx + 1).blue(),);
        }
        println!();
    }
}

fn analyze_ast(asts: Vec<Ast>) {
    for ast in asts {
        println!("{}", format!("{ast:#?}").blue());
    }
}

fn maybe_expand_bytecode(proc: ByteCodeProc) -> ByteCodeIter {
    let proc = Arc::new(proc);
    let mut iter = ByteCodeIter::from_proc(proc.clone());
    if iter.clone().count() == 1 {
        let instruction = iter.next().unwrap();
        match instruction {
            Instruction::GetVal(sym) => {
                let maybe_proc = Vm::singleton()
                    .get_value(sym)
                    .as_ref()
                    .and_then(Val::as_bytecode_proc);
                if let Some(proc) = maybe_proc {
                    return ByteCodeIter::from_proc(proc);
                }
            }
            Instruction::PushVal(val) => {
                if let Some(proc) = val.as_bytecode_proc() {
                    return ByteCodeIter::from_proc(proc);
                }
            }
            _ => (),
        }
    }
    ByteCodeIter::from_proc(proc)
}
