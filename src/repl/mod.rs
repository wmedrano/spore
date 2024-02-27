use std::rc::Rc;

use anyhow::{bail, Result};
use colored::Colorize;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;

use crate::parser::ast::{Ast, ParseAstError};
use crate::vm::compiler::Compiler;
use crate::vm::environment::Environment;
use crate::vm::types::instruction::Instruction;
use crate::vm::types::proc::{ByteCodeIter, ByteCodeProc};
use crate::vm::types::symbol::Symbol;
use crate::vm::types::Val;
use crate::vm::Vm;

pub mod command;

pub struct Repl {
    env: Environment,
    editor: DefaultEditor,
    repl_input: String,
    expression_count: usize,
}

impl Repl {
    pub fn new() -> Result<Repl> {
        let editor = DefaultEditor::new()?;
        Ok(Repl {
            env: Vm::with_builtins().build_env(),
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
        let input = std::mem::take(&mut self.repl_input);
        let (cmd, expr) = command::parse_command(input.as_str());
        let asts = match Ast::from_sexp_str(expr) {
            Ok(ast) => ast,
            Err(err) => {
                bail!("{}", err.display_with_context(expr));
            }
        };
        match cmd {
            "" => eval_asts(asts, &mut self.env, &mut self.expression_count),
            ",ast" => {
                for ast in asts {
                    println!("{}", format!("{ast:#?}").blue());
                }
            }
            ",bytecode" => analyze_bytecode(&mut self.env, asts),
            unknown => bail!("unknown command {unknown}"),
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

fn eval_asts(asts: Vec<Ast>, env: &mut Environment, expr_count: &mut usize) {
    for ast in asts {
        let res = Compiler::new(env)
            .compile_and_finalize(&ast)
            .and_then(|bc| env.eval_bytecode(bc.into(), &[]));
        match res {
            Ok(Val::Void) => (),
            Ok(v) => {
                *expr_count += 1;
                let sym = Symbol::from(format!("${expr_count}"));
                let _ = env.globals.insert(sym.clone(), v.clone());
                println!("{} = {}", sym.to_string().cyan(), v);
            }
            Err(err) => println!("{}", err.to_string().red()),
        }
    }
}

fn analyze_bytecode(env: &mut Environment, asts: Vec<Ast>) {
    for ast in asts {
        let proc = match Compiler::new(env).compile_and_finalize(&ast) {
            Ok(b) => b,
            Err(err) => {
                println!("{}", err.to_string().red());
                continue;
            }
        };
        let bytecode = maybe_expand_bytecode(env, proc);
        for (idx, bc) in bytecode.enumerate() {
            println!("  {:02} - {bc}", format!("{:02}", idx + 1).blue(),);
        }
        println!();
    }
}

fn maybe_expand_bytecode(env: &mut Environment, proc: ByteCodeProc) -> ByteCodeIter {
    let proc = Rc::new(proc);
    let mut iter = ByteCodeIter::from_proc(proc.clone());
    if iter.clone().count() == 1 {
        let instruction = iter.next().unwrap();
        match instruction {
            Instruction::GetVal(sym) => {
                if let Some(Val::ByteCodeProc(bc)) = env.globals.get(&sym) {
                    return ByteCodeIter::from_proc(bc.clone());
                }
            }
            Instruction::PushVal(Val::ByteCodeProc(bc)) => {
                return ByteCodeIter::from_proc(bc.clone());
            }
            _ => (),
        }
    }
    ByteCodeIter::from_proc(proc)
}
