use std::collections::HashMap;
use std::rc::Rc;

use anyhow::{bail, Result};
use colored::Colorize;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;

use crate::parser::ast::{Ast, ParseAstError};
use crate::vm::compiler::Compiler;
use crate::vm::debugger::TraceDebugger;
use crate::vm::environment::Environment;
use crate::vm::ir::CodeBlock;
use crate::vm::types::instruction::Instruction;
use crate::vm::types::{
    proc::bytecode::{ByteCodeIter, ByteCodeProc},
    symbol::Symbol,
    Val,
};
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
            env: Vm::new().build_env(),
            editor,
            repl_input: String::new(),
            expression_count: 0,
        })
    }

    /// Run the REPL.
    pub fn run(&mut self) -> Result<()> {
        println!(
            "{welcome}\n  {repo_link}\n",
            welcome = "Welcome to Spore!".cyan(),
            repo_link = "https://github.com/wmedrano/spore".cyan()
        );
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
        let asts = || match Ast::from_sexp_str(expr) {
            Ok(ast) => Ok(ast),
            Err(err) => {
                bail!("{}", err.display_with_context(expr));
            }
        };
        match cmd {
            "" => eval_asts(asts()?, &mut self.env, &mut self.expression_count, false),
            ",tokens" => {
                for token in crate::parser::lexer::tokenize(expr) {
                    println!("{token:?}");
                }
            }
            ",ast" => {
                for ast in asts()? {
                    println!("{}", format!("{ast}").blue());
                }
            }
            ",ir" => {
                for (idx, ast) in asts()?.iter().enumerate() {
                    let codeblock = CodeBlock::with_ast(
                        Some(format!("{idx}")),
                        HashMap::new(),
                        std::iter::once(ast),
                    )?;
                    println!("{}", format!("{codeblock:#?}").blue());
                }
            }
            ",bytecode" => analyze_bytecode(&mut self.env, asts()?),
            ",trace" => eval_asts(asts()?, &mut self.env, &mut self.expression_count, true),
            unknown => bail!(
                "unknown command {unknown}, expected one if {:?}",
                [",tokens", ",ast", ",ir", ",bytecode", ",trace"]
            ),
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

fn eval_asts(asts: Vec<Ast>, env: &mut Environment, expr_count: &mut usize, trace: bool) {
    for ast in asts {
        let mut maybe_trace = if trace {
            Some(TraceDebugger::default())
        } else {
            None
        };
        let res = Compiler::new(env)
            .compile("repl-eval".to_string(), &ast)
            .and_then(|bc| match maybe_trace.as_mut() {
                Some(t) => env.eval_with_debugger(bc.into(), &[], t),
                None => env.eval_bytecode(bc.into(), &[]),
            });
        if let Some(trace) = maybe_trace {
            println!("{trace}");
        }
        match res {
            Ok(Val::Void) => (),
            Ok(v) => {
                *expr_count += 1;
                let sym = Symbol::from(format!("${expr_count}"));
                env.set_global(sym.clone(), v.clone());
                println!("{} = {}", sym.as_str().to_string().cyan(), v);
            }
            Err(errs) => {
                for err in errs.chain() {
                    println!("{}", err.to_string().red());
                }
            }
        }
    }
}

fn analyze_bytecode(env: &mut Environment, asts: Vec<Ast>) {
    for ast in asts {
        let proc = match Compiler::new(env).compile("repl-analyze-bytecode".to_string(), &ast) {
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
                if let Some(Val::ByteCodeProc(bc)) = env.get_global(&sym) {
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
